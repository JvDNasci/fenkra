# Benchmarking: manufatura aditiva, simulação de manufatura, corte de tubos e conversão de malha

**Escopo e honestidade das evidências.** Pesquisei na web (buscas e leitura de páginas) e li os 13 itens no `docs/backlog/backlog.md`. Nenhum arquivo foi criado ou alterado, e não rodei checagens de código. Limites que afetam a confiança:

- Várias páginas-chave bloquearam a leitura: Lantek (404), The Fabricator (403), fornecedor de CAD avançado Blog sobre face groups (403) e wiki Bambu (402). Nesses casos usei os resumos das buscas.
- Preços públicos são raros e conflitantes. Onde não achei, escrevo "sob cotação".
- Itens marcados *(conhecimento prévio, verificar)* não vieram de fonte lida nesta sessão e precisam de confirmação antes de virarem requisito.
- Não achei nenhuma documentação pública do "parede que governa ponto a ponto" do B155. Isso pode ser diferencial, ou só falta de documentação pública dos concorrentes. Também não achei dado de mercado de corte de tubos, nem do Brasil.

---

## 0. Resumo executivo

1. **Tubos (B155 e B163) são o melhor nicho de diferenciação.** CAD avançado A Weldments, CAD avançado C Frame Generator e SigmaTUBE têm cópia/entalhe, mas tratam a parede como um corte geométrico único ou delegam o chanfro à máquina. Em nenhuma fonte achei compensação automática interna/externa por ponto com um orçamento de folga. A forma mais defensável de entregar isso é um **orçamento de folga de raiz** (máxima abertura admissível) que escolhe a parede ponto a ponto e avisa quando só um chanfro resolve (seção B155).
2. **Manufatura aditiva (B164–B166, B119, B120) disputa com ferramentas fortes de preparo (CAD avançado D, Magics, Netfabb) e com slicers gratuitos.** O Fenkra ganha por partir do **B-rep paramétrico**: faces analíticas dão análise de balanço e furos exata, e a preparação vira objeto não destrutivo na árvore. Não deve virar fatiador. O fatiador externo entra por linha de comando (PrusaSlicer, Orca, Bambu, CuraEngine são AGPL, então só como processo).
3. **Malha→STEP (B169, B158) é difícil e os líderes falham com frequência.** Os próprios fóruns da fornecedor de CAD avançado chamam a conversão de "incrivelmente difícil". O MVP honesto é **facetado + fusão de coplanares + reconhecimento de primitivas com B-rep híbrido** (faces analíticas e faces triangulares onde não houver ajuste). NURBS automático por regiões (modo 3) deve virar item separado e arriscado.
4. **Simulação de manufatura (B167 e B168) está grande demais.** Recomendo quebrar em dobra de chapa, acessibilidade de tocha e montagem. Dobra tem o mercado mais claro (HACO, Almacam, Radan e MachineWorks vendem só isso), e o manequim (B152) deve entrar depois.
5. **Fundação comum que falta no backlog:** um *corpo de malha* de primeira classe na árvore, uma biblioteca JSON de processos/máquinas/ferramentas e o *corpo para impressão derivado*. Isso está em "novos" no JSON final.

---

## 1. Bibliotecas e padrões transversais (licença × LGPL-2.1-or-later)

| Item | Licença | Uso no Fenkra | Observação |
|---|---|---|---|
| OpenCascade (OCCT) | LGPL-2.1 com exceção *(conhecimento prévio)* | Núcleo B-rep, sewing, `ShapeUpgrade_UnifySameDomain`, `ShapeFix`, `BRepBuilderAPI_MakeShapeOnMesh` *(verificar versão)* | Já é a base do Fenkra. |
| lib3mf | BSD-2-Clause (verificado) | Validação/escrita de 3MF | Compatível. Ligação via C ABI. Escrita própria zip+XML é alternativa. |
| 3MF Beam Lattice 1.2.0 e Materials 1.2.1 | Especificações abertas, royalty-free (verificado); 3MF virou ISO/IEC 25422:2025 | Export de vigas e multicor | Slicers hobby podem ignorar Beam Lattice *(verificar)*. |
| Manifold (elalish) | Apache-2.0 (verificado) | Booleanas de malha com saída garantidamente manifold; há port Rust nativo `manifold-rust` e binding `manifold-csg` (externos, maturidade a avaliar) | Apache-2.0 não é compatível com GPLv2-only. Com "LGPL-2.1-or-later" dá para distribuir sob LGPL-3+, mas confirme no ADR de licenças. |
| OpenVDB | Apache-2.0 (verificado, relicenciado de MPL-2.0) | Grades esparsas, nível de superfície, lattice implícito | C++ pesado. Alternativa: grade esparsa própria em Rust. |
| libigl | MPL-2.0 no núcleo; o repositório lista também GPL-3.0 (verificado) | Reparo, parametrização, segmentação | Auditar cada módulo antes de usar. |
| CGAL | Dual GPL/LGPL **por pacote**; a maior parte dos algoritmos é GPL (verificado) | Segmentação, remeshing | **Evitar linkagem.** Só processo externo. |
| Open3D, PCL | MIT / BSD *(conhecimento prévio; a página lida não declara)* | Nuvem de pontos, RANSAC, registro | Peso de dependência alto. Em Rust, RANSAC e ajuste de primitivas são viáveis de implementar. |
| OMPL | BSD-3-Clause (verificado) | Planejamento de caminho de montagem (RRT) | Mantido por terceiros; avaliar. |
| parry3d | Apache-2.0 *(conhecimento prévio)* | Colisão em Rust (BVH, cone/cilindro × malha) | Natural para acessibilidade de tocha. |
| Instant Meshes | BSD *(conhecimento prévio)* | Quad remeshing para o modo NURBS | Opcional. |
| CuraEngine | AGPL-3.0 (verificado) | Fatiador externo | Só processo externo. CLI: `CuraEngine slice -j settings.json -l modelo.stl -o saída.gcode`. |
| PrusaSlicer, OrcaSlicer, Bambu Studio | AGPL-3.0 *(conhecimento prévio)* | Abrir 3MF por linha de comando | Só processo externo, instalação do usuário. |
| FreeCAD | LGPL *(conhecimento prévio)* | Ideias e código do Part/Mesh | Código LGPL pode ser reaproveitado com atribuição. |

**Padrões úteis:** 3MF (núcleo, Materials, Beam Lattice), DSTV NC1 para perfis estruturais (furos e recortes) e STEP/IGES/SAT como entrada de CAM de tubo *(os dois últimos de conhecimento prévio e das páginas da Lantek)*, ISO 9013 (qualidade de corte térmico) *(conhecimento prévio)*. **BTL é formato de madeira** (a CAD avançado o descreve como padrão do setor madeireiro), então não serve para tubos.

---

## 2. B155 — Corte de encaixe normal entre membros (laser/plasma em tubos)

### Quem oferece
- **CAD avançado A Weldments, *Trim/Extend***. Tipos de canto: End Trim, End Miter, End Butt1 e End Butt2. Em mitras há Angle Bisector (perfis diferentes) e Full Flush (perfis iguais). A ordem de aparo é configurável (ex.: `1` para três vias). Tampas são outro recurso. Junta curva ou mitra não padrão exige superfície como ferramenta de aparo. O recurso corta pelo corpo vizinho. É incluído nas licenças Professional/Premium *(conhecimento prévio)*.
- **CAD avançado C Frame Generator.** Tratamentos de ponta são cumulativos: *Miter*, *Notch*, *Corner Joint*. O Notch tem opções "Extend Notch Profile" e "Perpendicular Cut" (para entalhe de tubo a laser). Limites documentados: membros curvos não são aparados por superfície curva. Fórum: entalhe prévio faz o Trim/Extend encurtar o lado errado.
- **Lantek Flex3d Tubes.** Chanfros completos ou parciais em qualquer ponto do tubo, simulação de máquina e sequência (avança/volta/pausa), pontes/microjuntas, qualidade de corte por parâmetro. Licenciamento não público.
- **SigmaTUBE (SigmaNEST/SigmaTEK).** Dentro do CAD avançado A. Varre a montagem, identifica tubos e agrupa por perfil/material. *4-Axis Compensation* gera caminho compensado que remove material para a solda encaixar **sem chanfro real** (máquinas sem cabeçote de chanfro). *Bevel Cutting* gera 5 eixos. *TrueShape* encaixa mitras/copes no aninhamento. Módulos: AutoNest, TrueShape, Techno e PowerPack (chanfro opcional nos dois primeiros, incluído nos outros).
- **HGG ProCAM** (máquinas de perfil): editor DSTV, preparação de solda ao longo da linha de ajuste de bocal, plug-in com SDS/2/CAD avançado que detecta furos e entalhes de tubo. Os dados vêm de reprints de comunicado do fabricante.
- **Fabricantes de máquina:** TRUMPF (software de tubo "Programming Tube"), BLM, Messer (fábrica em Jundiaí), Bystronic *(não verificado)*.

### Como funciona no detalhe
O fluxo dos melhores é: modelo 3D ou montagem → identificação do tubo e do corte → correção de colisão e qualidade → aninhamento → simulação → NC da máquina. O problema geométrico, segundo o fórum CADTutor, é que subtrair um tubo do outro dá uma parede que **afunila de t até zero** do lado interno ao externo, e ninguém resolve a "mitra com parede cheia" de forma automática nesses fóruns. A física do laser acrescenta o *foco mais negativo em tubo que em chapa* (cerca de −2,5 a −3 mm numa parede de 3 mm, contra −1,5 em chapa), crescente com diâmetro menor e parede maior, e o uso da parede mais grossa em retangular de parede desigual (guia de parâmetros GWEIKE).

### Elogios e reclamações
Não achei avaliações independentes: o Capterra tem 1 avaliação de 5/5 para o Flex3d. O que há são limitações documentadas (CAD avançado C com curvos; ordem de aparo) e a queixa recorrente de parede afunilada nas mitras.

### Bibliotecas e padrões
OCCT (booleanas, `BRepAlgoAPI`, offset de curva), DSTV NC1 para perfis, STEP do tubo cortado como saída para CAM de tubo, DXF para o desenvolvimento.

### Recomendação para o Fenkra
**Fluxo (CAD avançado B):** o comando entra na árvore como *"Corte de encaixe"* sob o membro cortado, com publicações (curva de contato, face de referência, parede governante). Painéis em ordem **suporte → modo → parâmetros → avançado**:
1. *Suporte:* membro a cortar e membro(s) de referência (referência estável a faces, ADR 0010).
2. *Modo:* **Automático por orçamento de folga** (padrão), *Parede interna*, *Parede externa*, *Média* (esses três como avançado).
3. *Parâmetros:* abertura de raiz máxima (folga admissível), kerf, ângulo de chanfro desejado (0 = sem chanfro).
4. *Avançado:* tolerâncias reais do tubo (diâmetro, parede ±10%, raio de canto em retangular), superfície de referência do desenvolvimento.

**Algoritmo proposto (hipótese a validar):** em cada ponto da curva de contato, comparar a folga que sobraria se o corte tangenciasse a parede interna com a folga se tangenciasse a externa. Escolher a parede cuja folga fique **dentro do orçamento**, com histerese e suavização nas trocas para evitar degraus. Onde nenhuma cabe, marcar o trecho em vermelho e sugerir chanfro (cabeçote 5 eixos) ou aumentar o orçamento. O 3D mostra a parede em vigor por cor (já previsto no item). A mesma métrica de folga serve de **teste de aceite**.

**MVP (1ª versão):**
- Tubo redondo × redondo (boca de lobo) perpendicular e oblíquo.
- Retangular × retangular (entalhe) e perfil sobre chapa.
- Modo automático, kerf, desenvolvimento em DXF (curva planificada pela superfície de referência), lista de corte.
- Recálculo quando o esqueleto muda.

**Depois:** excêntricos, tubos múltiplos concorrentes no mesmo nó, perfis abertos (U, L, I), chanfro variável calculado, saída DSTV e STEP do tubo cortado para CAM de tubo, tampa de extremidade.

**Integrações:** B163 (caminho da tocha usa a mesma curva), Estrutural (esqueleto e lista de corte), B112/B113 (chanfro e espessura), B119 (nada), Visualizador B156 (animar corte).

**Riscos técnicos:**
- Estabilidade das curvas de contato com booleanas OCCT em tangências.
- Tubos reais não são ideais (ovalização, costura).
- Cantos arredondados de retangular multiplicam casos.
- Definição de "desenvolvimento" em parede grossa (qual superfície é a neutra).

**Testes de aceite (Auto + Visual):**
- Para 8 casos de referência, o volume de interferência entre membro cortado e hospedeiro é ≤ ε e a folga máxima fica dentro do orçamento.
- O 3D destaca a parede em vigor por trecho.
- Re-envolver o desenvolvimento reproduz a curva 3D com erro ≤ 0,01 mm.
- Alterar o esqueleto atualiza o corte e a lista de corte.
- DXF abre em visualizador externo com comprimento de corte batendo com o relatório.

**Tamanho:** 350 k é realista só para redondos e retangulares. Perfis abertos e saídas de máquina somam 150 k à parte.

---

## 3. B163 — Caminho de corte em tubo (laser/plasma rotativo) e representação

### Quem oferece
Lantek Flex3d Tubes (simulação com avanço/retrocesso, pontes e microjuntas, chanfro completo ou parcial), SigmaTUBE (toolpath automático/manual por qualidade de corte, compensação de 4 eixos, bevel 5 eixos, detecção de colisão específica da máquina, *common-line*, relatórios), HGG ProCAM (editor DSTV, preparação de solda, parâmetros de raiz e marcação), TRUMPF "Programming Tube" (processos 3D automatizados). Preços: não públicos, módulos opcionais por pacote no SigmaTUBE (verificado).

### Como funciona
Cinemática típica de tubo rotativo: mandril gira o tubo (**A**), carro move ao longo do tubo (**X**), cabeçote ajusta Y/Z e, quando é de chanfro, inclina (B/C) *(conhecimento prévio)*. O programa de peça define ordem de cortes, entradas/saídas, microjuntas (para a peça não cair antes da hora), qualidade (parâmetros por trecho) e correção de colisão da tocha com o próprio perfil. A saída é NC específico da máquina.

### Elogios e reclamações
Sem reviews independentes. Os fabricantes insistem em redução de refugo (aninhamento comum), em simulação e no corte de chanfro. O vazio no mercado é a **portabilidade**: cada fabricante tem seu pós. Isso reforça a decisão do backlog de **não** fazer G-code por máquina.

### Recomendação
**Fluxo:** *Caminho de corte* na árvore, filho do corte do B155. Painéis: *suporte* (cortes a incluir) → *modo* (normal à superfície / radial / inclinado pelo chanfro) → *parâmetros* (início, lead-in/lead-out, microjuntas, sentido, ordem, kerf) → *avançado* (colisão da tocha, tolerâncias).

**MVP:**
- Curva 3D do corte limpo (do B155) amostrada em (X, A, bisel).
- Vetor da tocha por ponto, início/entrada/saída.
- Microjuntas, ordem de cortes (último corte separa a peça).
- Tabela por ponto e CSV.
- Vista desenvolvida com o caminho e animação do cabeçote.
- Relatório de comprimento/tempo.

**Depois:** detecção de colisão tocha × perfil (parry3d), *common-line*, aninhamento de tubo, pós por máquina (pelo menos um, via parceria), 5 eixos completos.

**Integrações:** B155 (curva), Visualizador B156 e animações, B112/B113 (espessura), lista de corte.

**Riscos:**
- Singularidades da rotação em retangulares.
- Tubo longo sem apoio (flecha).
- Ordem de corte que deforma o tubo.
- Velocidade e tempo realistas dependem da máquina; entregar "estimativa configurável", não promessa.

**Aceite:** CSV com X crescente/A contínuo (sem saltos > limite), vetor da tocha dentro do cone permitido, microjunta presente em cada contorno fechado, relatório de comprimento igual ao da curva ±0,1%, animação sem colisão nos casos de referência.

**Tamanho:** 400 k mantido. Colisão e *common-line* vão para um item posterior de ~150 k.

---

## 4. B164 — Bancada de Manufatura aditiva

### Quem oferece
- **Fusion Additive Build Extension.** Define a impressora e os parâmetros, organiza as peças na plataforma (*Additive Arrange*), cria suportes. O estudo de **orientação automática** classifica por área de suporte, com "Smallest Rotation" configurável (15° no tutorial). **Não considera o volume de construção:** resultados fora da mesa ficam em vermelho. Exige assinatura do CAD avançado D, mensal ou anual. Preços divulgados por revendas: US$ 1.120 de lista e US$ 1.327,20 como venda (conflitantes, datados).
- **fornecedor de CAD avançado Netfabb.** Premium: empacotamento 2D/3D, simulação PBF de escala de peça, reparo, suportes configuráveis. Ultimate: lattice interno (marcado "só Ultimate"), automação com fatiamento e relatórios. Simulação ilimitada é produto separado.
- **Materialise Magics.** Líder de preparo industrial. Módulos de suporte (manual, semi e automático por "scaffold"), Nester (posicionamento manual a totalmente automático), Lattice Module unificado (Magics 28), Analyze Build com análise de parede, colisão e interlocking. Valores soltos em revenda: US$ 199/mês e US$ 1.995/ano, sem edição identificada.
- **Formware.** Reorientação mínimizando faces voltadas para baixo, estruturas de piso, verificação de erros de malha, lattice de infill. Licença pessoal € 129,95 (3DNatives) ou US$ 154,95 (site), comercial € 299,95; inconsistentes.
- **Slicers:** OrcaSlicer orienta por área de balanço, contato de base, interface de suporte e complexidade de contorno. **Meshmixer** teve desenvolvimento encerrado em 2021, mas segue lembrado por suportes e reparo.
- **nTop:** modelagem implícita por campos, licença proprietária, preço não público.

### Como funciona no melhor (CAD avançado D/Magics)
Máquina → material/parâmetros → posicionamento → orientação ranqueada → suportes → saída. A inteligência está no *ranking multicritério* e em suportes por física (Magics/Materialise na linha metal).

### Elogios e reclamações
Fusion: bom ponto de entrada, mas orientação ignora volume. Magics: considerado o padrão, caro e vinculado a módulos. Bambu: booleanas "bugadas" e cortes só globais (plano infinito). Falta dos concorrentes: partem de **malha**, perdem parametria.

### Recomendação
**Fluxo:** *Preparo de impressão* é um objeto na árvore (referencia o corpo e as faces por referência estável). Painéis: *suporte* (corpo(s)) → *modo* (processo: FDM, SLA/DLP, SLS/MJF, metal) → *parâmetros* (máquina com volume, camada, ângulo crítico, parede mínima) → *avançado* (pesos do ranking).

**MVP:**
- Biblioteca JSON de máquinas e processos editável.
- Análise de balanço por **faces do B-rep** com mapa de cor e ângulo crítico por processo.
- Orientação por amostragem (normais de faces planas + esfera de Fibonacci) pontuando área de suporte, contato com a mesa, altura e cabimento no volume.
- Parede fina e furo mínimo (reusa B112/B113).
- Estimativa de massa e material.
- Tempo e material reais **via fatiador externo** (CLI lê a estimativa do G-code), sem escrever fatiador próprio.
- Export 3MF (B119) e abertura no fatiador.

**Depois:** geometria de suportes (árvore e linear), resistência por eixo, simulação de distorção em metal (não competir com Netfabb Simulation), nesting de várias peças.

**Integrações:** B119 (I/O), B165, B166, B120, B112/B113, a nova biblioteca de processos/máquinas.

**Riscos:**
- Tessellação define o mapa de balanço; usar a face analítica quando existir.
- Cálculo de tempo depende do perfil do fatiador do usuário.
- Escopo explosivo de suportes.

**Aceite:** em peças-padrão (cubo, L, pino, chapa com furo horizontal), o ranking prefere a orientação esperada; mapa de balanço marca exatamente as faces > ângulo crítico; peça maior que o volume nunca é a 1ª sugestão; 3MF abre no PrusaSlicer/Orca com a unidade correta.

**Tamanho:** 450 k é otimista incluindo suportes. Recomendo cortar para orientação, análise e estimativa (~350 k) e abrir um item de suportes de ~300 k.

---

## 5. B165 — Partição, encaixes inteligentes e cores

### Quem oferece
- **PrusaSlicer Cut.** Modos planar e *dovetail*. Conectores *plug*, *dowel* e *snap*, com estilo, forma, profundidade, tamanho e rotação. *Flip cut plane*. Resultado em dois objetos ou "partes" (não vale em dovetail). Regiões de corte escolhidas por clique. Limites: colocação só por clique e dovetail incompatível com partes.
- **Bambu Studio.** Mesmos tipos de conector. A tolerância encolhe o pino em todos os lados (relato de usuário em fórum); usuários também reportam "Place on Cut" instável e booleanas problemáticas. O corte é um plano infinito.
- **CAD avançado D, Meshmixer.** Dividir corpo por plano/superfície; sem biblioteca de conectores *(conhecimento prévio, verificar)*.
- O comportamento da tolerância no PrusaSlicer é **contraditório entre relatos** (alarga o furo versus nenhum efeito).

### Recomendação
**Fluxo:** *Partição* na árvore (plano ou superfície de corte como suporte) → *conectores* (tipo, posição por croqui/restrição na face de corte) → *folga* vinda do processo (B164) → *numeração* por relevo de texto.

**MVP:**
- Corte planar por plano da árvore.
- Pino separado (*dowel*) e encaixe macho/fêmea com folga do processo.
- Sugestão automática de corte (plano candidato por menor seção, face plana de apoio e cabimento no volume).
- Numerar e listar partes.
- Divisão por cor: pintura de faces vira corpos separados e 3MF (extensão Materials, que cobre cores e multimaterial).

**Depois:** cauda de andorinha, *snap-fit* (com fórmulas de deformação do braço), inserto térmico (diâmetros de catálogo), superfície de corte livre, 3MF multimaterial compatível com AMS/MMU, que usa metadados próprios dos fatiadores *(verificar)*.

**Riscos:** divisão de corpo com filetes e roscas, conectores que colidem com furos, 3MF multicor entre fatiadores.

**Aceite:** as partes remontadas ocupam o volume original (diferença de volume ≈ folga × área); pino entra no furo com folga igual à configurada; peça maior que a mesa vira N partes que cabem; cada parte tem número no relevo e na lista.

**Tamanho:** 400 k é razoável para o MVP acima. Snap-fit e multicor avançado somam ~150 k à parte.

---

## 6. B166 — DfAM avançado e compensação

### Quem oferece e como
Magics 28 (lattice unificado e análise), Netfabb Ultimate (otimização), nTop (campos). Regras públicas de DfAM:
- Balanço > ~45° exige suporte; furo horizontal usa **gota ou losango**.
- Parede mínima ~0,8 mm (FDM), 0,4–1,0 mm (SLA), 0,7–1,0 mm (SLS/MJF), 0,8–1,2 mm (DMLS).
- Folga de montagem sugerida: 0,3 mm (FDM), 0,2 mm (SLA), 0,15 mm (SLS), 0,2 mm (MJF).
- Furos de escape de 3–5 mm em peças ocas.

Esses números variam por fonte, então devem ser **dados editáveis**, nunca constantes no código.

### Recomendação
O ponto de arquitetura é o **corpo para impressão derivado**: a compensação (escala por eixo, folga em furos e pinos, gota em furos horizontais) vira um corpo derivado do projeto, preservando o corpo original e as exportações de engenharia. A contração é calibrada pelo usuário: o Fenkra gera um **cupom de teste**, o usuário mede e grava o fator no perfil da máquina. Faixas típicas de contração *(conhecimento prévio, a validar por calibração)*: PLA ~0,2%, PETG ~0,3%, ABS ~0,4–0,8%, nylon SLS ~3%.

**MVP:** gota/losango automático em furos horizontais, folga por processo em pares de furo/pino, compensação linear por eixo, relatório de imprimibilidade com nota por peça reaproveitando o **motor de regras do B112** (perfil JSON do processo).

**Depois:** alívio de massa, pontes (reduzir vão), conversão de furos horizontais em furos escalonados, integração com lattice (B120).

**Riscos:** a compensação muda geometria que o usuário também vai medir; precisa de marcação clara no 3D e reversão.

**Aceite:** furo horizontal com Ø 6 mm vira gota com diâmetro nominal igual e ponta ≤ ângulo crítico; escala por eixo aplicada ao corpo derivado altera o BBox na proporção; relatório lista violações com IDs de face.

**Tamanho:** 350 k é adequado com o motor de regras reaproveitado. Sem o B112 pronto, vira 450 k.

---

## 7. B167 — Simulação de manufatura (máquinas, ferramentas e acesso)

### Quem oferece
- **fornecedor de CAD avançado Process Simulate (Weld).** Projeção de soldas nas peças, assistente de busca de pinça, projeção de cordão de arco, alinhamento da tocha, validação de pinça e teste de alcance de robô. O foco é robótica industrial e licenças de alto valor (preço não público).
- **Process Simulate Human.** Postura, alcance, visão, folga de mão, OWAS, RULA, NIOSH e gasto energético. REBA não aparece nas listas do fornecedor. O Jack (Tecnomatix) foi descontinuado segundo uma revenda *(verificar)*.
- **Dobra de chapa.** HACO hacobend pro 3D: simulação 3D com detecção de colisão e detecção automática de dobras de STEP/IGES. Almacam Bend: sequência e ferramentas com acessibilidade e risco de colisão a cada passo. Radan Radbend: sequência, ferramentas e batentes automáticos. MachineWorks: colisão em colocação, dobra e retirada, e simulação de retorno elástico. Pesquisa: o problema é combinatório, resolvido com busca best-first e colisão. Fabricantes (TRUMPF, Bystronic, Amada) têm software preso às suas máquinas. Não achei dobra de sequência nativa em CAD avançado A/CAD avançado C nas buscas *(verificar)*.
- **DELMIA** (veja B168).

### Recomendação
O item de 450 k mistura cinco problemas diferentes. Proponho dividir:
1. **Dobra de chapa** (sequência, punção/matriz, colisão da aba com máquina e ferramenta, aba mínima, retorno elástico estimado). É o mercado mais claro (serralherias e caldeirarias) e usa a chapa metálica que o Fenkra já tem.
2. **Acessibilidade de tocha** (MIG/TIG): para cada ponto do cordão, amostrar direções da tocha (ângulo de trabalho e de deslocamento) e testar o cone do bico contra a malha com BVH; resultado: cone de direções viáveis colorido no 3D e pontos inviáveis marcados.
3. **Furadeira/fresa:** comprimento da broca e acesso perpendicular (incorporar ao B112).
4. **Pinça de ponto, puncionadeira:** depois.
5. **Manequim operador:** depende do B152 e fica para depois.

**Biblioteca de ferramentas e máquinas** (JSON editável) é item próprio e compartilhado com B112/B164/B166.

**MVP da dobra:** sequência manual com validação de colisão em cada dobra (aba × punção, aba × corpo da máquina), sugestão automática por busca limitada, retorno elástico por fórmula analítica *(conhecimento prévio, a calibrar por material)*.

**Riscos:** modelar a dobradeira real (geometria de punção, matriz e mesa) exige dados do fabricante; o retorno elástico real depende do lote do material.

**Aceite:** caixa de 4 abas com aba curta detecta colisão na 4ª dobra se a ordem for errada e sugere ordem válida; cone de tocha em um canto interno de 90° exclui as direções que batem na parede; relatório marca operações inviáveis com a face.

**Tamanho:** 450 k deve virar ~400 k (dobra) + ~300 k (tocha) + ~200 k (biblioteca).

---

## 8. B168 — Sequência de montagem e ferramentas de montagem

### Quem oferece
**DELMIA Assembly Simulation Expert** gera trajetórias de montagem/desmontagem sem colisão a partir de ponto inicial, intermediário e final, folgas, ponto-guia, caixa delimitadora ou caminho-guia. Otimiza trajetórias irregulares e gera **volumes varridos** que atualizam quando o projeto muda. A tecnologia de planejamento de movimento (Kineo CAM) já foi integrada ao CAD avançado B DMU Fitting Simulator (2007). A pesquisa acadêmica filtra componentes por visibilidade e depois por alcance; o resultado é uma sequência local, não ótima global.

### Recomendação
**MVP:**
- Sequência editável a partir da explodida.
- Cada passo move uma peça por translação ou rosca (eixo helicoidal) e calcula colisão e volume varrido.
- A ferramenta (chave, parafusadeira) vira volume de acesso (cilindro/soquete) testado no ponto de fixação.
- Animação e relatório por passo, exportação ao Visualizador (B156) e às instruções animadas (B124).

**Depois:** planejamento automático (grafo de bloqueio direcional, RRT via OMPL) e manequim (B152) com RULA/REBA/NIOSH. Custo/benefício: manequim só faz sentido quando B152 existir.

**Riscos:** caminhos não retilíneos, tolerâncias de folga, tamanho do volume varrido em montagens grandes.

**Aceite:** bloqueio detectado ao inserir o pino antes do eixo; ordem inversa válida em desmontagem; volume varrido exportado como corpo visualizável.

**Tamanho:** 350 k mantido para MVP sem manequim.

---

## 9. B169 — Converter malha (STL/OBJ/3MF) em STEP sólido

### Quem oferece
- **CAD avançado D, *Convert Mesh*:** Faceted (uma face por triângulo), Prismatic (funde grupos de faces; exige *Generate Face Groups*) e Organic (exige Design Extension; resolução por precisão baixa a *precise* ou por número de faces; pré-processa furos). Operação Parametric (mantém histórico) ou Base Feature. Malha estanque vira sólido, aberta vira superfície. **Reclamações:** Prismatic falha ou devolve superfície, Organic falha com malha suja ou muito densa (usuários reduzem para ~10 mil triângulos), opção "Base Feature" difícil de achar. Um diretor de engenharia disse que é "uma conversão incrivelmente difícil".
- **FreeCAD:** *Part → Create shape from mesh* com costura (tolerância padrão 0,1), depois *Convert to solid* e *Refine*; **sem validação**, então reparar antes. Há tutorial de STL→STEP no GrabCAD.
- **CAD avançado A ScanTo3D:** *Mesh Prep Wizard* + *Surface Wizard* automático (por número de superfícies, para orgânico) ou guiado (regiões pintadas, para faces analíticas). Disponível nas edições Professional/Premium segundo fontes conflitantes.
- **Geomagic Design X** (Oqton/Hexagon): planos Go (a partir de US$ 1.900/ano), Plus (US$ 3.990 em revenda), Pro (US$ 6.930 em revenda, sob consulta no site); Pro tem assistentes guiados e automação; transferência ao vivo para CAD avançado A, CAD avançado G, CAD avançado F, CAD avançado J e CAD avançado C; licença travada em computador, dongle ou rede.
- **Mesh2Surface / QuickSurface:** plug-ins de CAD avançado I e produto próprio, preço sob cotação. O relatório de mercado cita Oqton, QUICKSURFACE, Artec e Mesh2Surface entre os principais.
- **CAD avançado B** (Digitized Shape Editor, Quick Surface Reconstruction) como referência do backlog.

### Elogios e reclamações
Elogio: fluxo guiado do Design X e do ScanTo3D para peças mecânicas. Reclamação dominante: conversões que falham ou produzem corpos que não aceitam operações; o usuário precisa de malha limpa e agrupada antes.

### Recomendação
**MVP honesto (B169 como descrito, modos 1 e 2):**
1. Importar STL/OBJ/3MF/PLY, solda de vértices, reparo básico (furos, normais, partes soltas), diagnóstico.
2. **Facetado:** `BRepBuilderAPI_Sewing`/`MakeShapeOnMesh` + fusão de faces coplanares (`UnifySameDomain`).
3. **Primitivas:** crescimento de regiões por curvatura/normal, ajuste de plano/cilindro/esfera/cone/toro por mínimos quadrados e RANSAC, geração das arestas pela **interseção das superfícies vizinhas**, B-rep **híbrido** (onde não houver ajuste, continuam faces triangulares), garantindo sólido válido.
4. Relatório de desvio malha × sólido (distância ponto-superfície com BVH).
5. STEP AP242 e continuação do modelo (filete, furo, booleana) sobre o resultado.

**Modo 3 (NURBS por regiões):** pesquisa, não produto. Se feito, via remeshing quad externo (Instant Meshes) e remendos; qualidade fraca fora de formas simples. **Separar em item próprio.**

**Integrações:** B158 (segmentação e ajuste são compartilhados), conversor de arquivos, lote.

**Riscos:** topologia na interseção de primitivas vizinhas; tolerâncias; malhas de milhões de triângulos; expectativa do usuário de "um sólido limpo".

**Aceite:** cubo furado em STL vira sólido com 6 faces planas e 1 cilíndrica reconhecidas (contagem de faces = esperado) e desvio máximo ≤ tolerância; STEP exportado reabre no FreeCAD como sólido válido; malha com furos recebe relatório e falha com mensagem útil.

**Tamanho:** 350 k cobre os modos 1 e 2 com malha limpa. Modo 3: +400 k separado. Reparo robusto: dentro do núcleo de malha novo.

---

## 10. B119 — Exportação 3MF e preparo de impressão

### Quem oferece
Todos os fatiadores abrem 3MF. O padrão (núcleo + Materials 1.2.1 + Beam Lattice 1.2.0) é livre e virou ISO/IEC 25422:2025. A lib3mf é BSD-2-Clause, com versões 2.4 e 2.5 listadas.

### Recomendação
- Escrever 3MF direto (zip + XML) para o núcleo, **validar com lib3mf em CI**. Se o binding Rust custar caro, a escrita própria é pequena. Unidade mm, tolerâncias de tesselação configuráveis (desvio linear e angular), objetos com múltiplos corpos, *build items* com transformação (posição na mesa), metadados (nome, número de peça), miniatura.
- **Verificação pré-impressão** (estanque, arestas não manifold, normais, partes soltas) antes de exportar.
- **Abrir no fatiador externo:** o usuário configura o caminho; detecção automática de PrusaSlicer, Orca e Bambu; processo separado.
- Orientação "assentar numa face" e mapa de balanço mínimo ficam aqui. O restante de orientação é do B164.

**Riscos:** fatiadores usam metadados 3MF próprios; Beam Lattice pode ser ignorado em slicers hobby *(verificar)*.

**Aceite:** 3MF abre no PrusaSlicer e Orca com escala e unidade corretas; lib3mf lê sem erro; peça com furo passante preserva topologia; cores por face exportam como Materials.

**Tamanho:** 250 k adequado, possivelmente 200 k com escrita própria.

---

## 11. B120 — Lattice e preenchimento implícito

### Quem oferece
**nTop** (campos implícitos, referência em lattice graduado; licença proprietária, preço não público), **Magics 28** (Lattice Module com vigas e lattice por fatias), **Netfabb Ultimate** (otimização de lattice), **Altair Inspire** (modelagem implícita com lattices dirigidos por campo, produto distinto). Para quem imprime por FDM, o *infill* do fatiador já resolve a maior parte; lattice importa em SLS/MJF/metal e em peças leves.

### Como funciona
Campo implícito de uma célula (TPMS: giroide `sin x cos y + sin y cos z + sin z cos x = t`, Schwarz-P, diamante), parede a partir do nível `t`; ou vigas (BCC, octeto). Intersecção com o corpo, casca externa, gradiente por campo (distância a uma face, carga).

### Recomendação
- Tipo de corpo novo na árvore (**corpo de malha** da seção de novos).
- MVP: TPMS uniforme e células de viga em região delimitada, casca e preenchimento, volume-alvo (fração).
- Superfície por marching cubes/dual contouring sobre grade esparsa (OpenVDB, ou grade esparsa própria em Rust).
- Export: malha triangulada e, para sistemas industriais, 3MF Beam Lattice.

**Riscos:** memória e tempo (milhões de triângulos), parede mínima por processo, manifold.

**Aceite:** fração de volume = alvo ±3%; espessura mínima de parede ≥ limite do processo; malha final estanque; export lido pela lib3mf.

**Tamanho:** 450 k deve ser dividido: TPMS uniforme (~250 k) e vigas + Beam Lattice + gradiente (~250 k).

---

## 12. B112 — Perfil de regras DFM (furo, bolso, chapa)

### Quem oferece
**CAD avançado A DFMXpress** (versão reduzida do DFMPro da HCL, incluído em todas as licenças): regras de fresa/furo (bolso profundo, feição inacessível, canto interno vivo), torno, chapa (distância furo–borda, raio de dobra) e injeção (parede mínima e máxima); tolerâncias ajustáveis. **Protolabs** tem DFM instantâneo para usinagem, injeção e impressão 3D (lista de regras não pública). aPriori e Xometry fazem DFM com custo *(conhecimento prévio)*.

### Recomendação
Motor de regras com **perfis JSON** por processo (furo, bolso, chapa, AM), cada regra com `id`, tipo de entidade (face, furo, aresta), limites, severidade e texto. Varredura devolve lista de violações com referência estável à face e realce 3D. É a base de B113, B166 (AM) e B167. **Aceite:** peça com furo a 0,5 mm da borda e parede de 0,3 mm gera duas violações com IDs corretos; editar o limite do perfil muda o resultado sem recompilar. **Tamanho:** 300 k mantido.

---

## 13. B113 — Espessura, saliência e canto vivo

### Quem oferece
**CAD avançado A Thickness Analysis** realça regiões grossas/finas a partir de uma espessura-alvo e gera relatório. DFMXpress cobre cantos vivos internos. Magics tem análise de parede.

### Recomendação
Espessura por **raio-bola máxima/ray casting** a partir de pontos sobre as faces (B-rep avaliável com tesselação adaptativa), mapa de cor com legenda e limite por processo (do perfil do B112). Saliência (*overhang*) vem da normal da face contra a direção de construção (B164). Canto vivo: arestas côncavas com raio abaixo do mínimo da ferramenta. **Aceite:** placa com degrau de 1 mm e 3 mm mostra duas faixas com erro ≤ 5%; o mapa só pinta o que viola o limite se o filtro estiver ativo. **Tamanho:** 200 k mantido.

---

## 14. B158 — Engenharia reversa: nuvem de pontos e malha para CAD

### Quem oferece
Geomagic Design X (preços na seção B169), CAD avançado A ScanTo3D, CAD avançado B Digitized Shape Editor/QSR, Fusion (Mesh workspace), Mesh2Surface/QuickSurface; códigos abertos: Open3D (MIT) e PCL (BSD), ambos com RANSAC de planos/cilindros *(conhecimento prévio)*.

### Recomendação
Esta bancada é a **casa** da malha: importação (STL/OBJ/PLY/E57), limpeza, decimação, preenchimento de buracos, segmentação por curvatura, ajuste de primitivas, extração de seções e croquis, comparação de desvio com mapa de cores. O B169 consome o mesmo ajuste. Ordem sugerida: núcleo de malha → segmentação/ajuste → seções e croquis → desvio → NURBS. Nuvem de pontos crua (E57, milhões de pontos) é custo à parte: amostragem, octree, normais.

**Aceite:** malha de uma peça com plano, cilindro e esfera segmenta em 3 regiões com ajuste de raio ±0,05 mm; seção em um plano vira polilinha convertível a croqui; desvio máx/médio coerente com o ground truth.

**Tamanho:** 450 k só é realista com o núcleo de malha pronto (item novo de ~300 k); caso contrário, o B158 absorve o núcleo e sobe para ~700 k.

---

## 15. Mercado

### Quem compra e por quê
| Nicho | Necessidade | Módulos do Fenkra |
|---|---|---|
| Caldeiraria, serralheria e estrutura metálica (tubos, perfis) | Cortar encaixes em laser de tubo, lista de corte, desenvolvimento | B155, B163, dobra (B167a) |
| Impressão 3D (prototipagem, oficinas, makers) | Preparo, partição, folgas, 3MF | B119, B164–B166, B120 |
| Manufatura industrial e DfAM metal | Orientação, suporte, lattice | B164, B166, B120 (concorre com Netfabb/Magics) |
| Engenharia reversa e manutenção | STL→STEP, desvio | B158, B169 |
| Aeronáutico | Montagem, acesso, rastreabilidade | B168, B167 |
| Educação | Software livre com ensino de DFM/DfAM/ergonomia | todos |

### Tamanho e dados
Os números divergem muito por metodologia:
- **Software de AM:** a AM Research cita cerca de US$ 2,4 bi (2024) chegando a mais de US$ 6,8 bi por volta de 2033. Outras consultorias dão desde US$ 4,25 bi até US$ 32,6 bi para 2025, com CAGR de 9,8% a 39%. Usar a ordem de grandeza (alguns bilhões) e não um valor exato.
- **Engenharia reversa 3D:** US$ 338,7 mi em 2025 → US$ 573,2 mi em 2035 (FMI, fonte única). O "reverse engineering software" amplo vai a US$ 1,4–1,8 bi.
- **Corte de tubos:** sem dado de mercado nas buscas. Há máquinas chinesas anunciadas por R$ 128.900 (tubo HN6010) e fabricantes locais (Messer em Jundiaí e Caxias do Sul), o que mostra oficinas pequenas comprando máquina e carentes de software acessível.

### Preços de referência (para posicionamento)
Fusion Additive Build Extension ≈ US$ 1,1–1,3 mil/ano; Geomagic Design X a partir de US$ 1.900/ano; Magics e SigmaTUBE sob cotação; Formware ≈ US$ 130–300. Os fabricantes de tubo cobram em módulos.

### Onde o Fenkra pode se destacar
1. **Corte de encaixe de tubo com folga controlada** acoplado à estrutura paramétrica e à lista de corte. Nenhum concorrente analisado liga o modelo estrutural ao caminho do laser com prova de folga.
2. **AM a partir do B-rep**, sem perder a parametria, com corpo para impressão derivado.
3. **Dobra de chapa e tocha em software livre**, com biblioteca de máquinas aberta, em um mercado dominado por pacotes presos a fabricante.
4. **STL→STEP honesto**: híbrido com relatório de desvio, em vez de prometer "sólido automático".
5. **Português e Brasil**: caldeiraria, normas e fabricantes locais.

Riscos de mercado: o Fusion cobre bem o hobby/pro de AM por assinatura; Magics/Netfabb dominam o industrial metal; Lantek/SigmaTUBE dominam o tubo com integração de máquina. O Fenkra não deve tentar G-code por máquina nem simulação térmica.

---

## 16. Fechamento (handoff)
- **Resultado:** relatório com 13 itens, mercado e atualizações propostas ao backlog.
- **Evidência:** buscas e leituras listadas abaixo; itens marcados *(verificar)* precisam de confirmação.
- **Arquivos alterados:** nenhum. **Checagens executadas:** nenhuma (pesquisa).
- **Falhas:** quatro páginas bloqueadas (Lantek 404, Fabricator 403, fornecedor de CAD avançado Blog 403, wiki Bambu 402); sem dados de mercado de corte de tubos; sem preço oficial de Magics, Lantek, SigmaTUBE, nTop e Mesh2Surface.
- **Riscos:** números de contração, ângulos críticos e folgas são faixas de blog e precisam ser calibrados; o algoritmo de "orçamento de folga" do B155 é hipótese de projeto a validar com casos reais.

---

## Fontes (URLs)

- https://global.channelonline.com/acme/ccs/Products/overview/M023449367
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.materialise.com/en/software/magics
- https://static.helpjuice.com/helpjuice_production/uploads/upload/image/5908/4049616/Magics_28_Manual.pdf
- https://help.prusa3d.com/article/cut-tool_1779
- https://forum.bambulab.com/t/cut-menu-in-studio/152029
- https://forum.bambulab.com/t/cut-tool-limitation-need-scalable-local-cut-not-full-plane/247598
- https://www.formware.co/slicer/features
- https://www.3dnatives.com/en/formware-3d-slicer-support-generation-software-for-3d-printing-211020244/
- https://docs.ntop.com/lessons/ntop-essentials/implicits-and-fields
- https://www.orcaslicer.com/wiki/print_prepare/prepare_auto_orient
- https://forum.bambulab.com/t/meshmixer-software/251480
- https://www.lantek.com/ae/lantek-flex3d-tubes
- https://ctemag.com/?p=101835
- https://www.sigmateksystems.com/en/sigmatube-sw
- https://www.sigmanest.com/en/supported-machine-types/tube
- https://www.thefabricator.com/thefabricator/product/cadcamsoftware/software-designed-to-maximize-throughput-on-3d-profiling-machines
- https://www.smacna.org/news/news-archive/article/2024/04/30/procam-software-developed-by-profilers-for-profilers
- (fonte em procedencia.md)
- https://www.cadtutor.net/forum/topic/37391-creating-tube-miter-correctly/
- https://www.gwklaser.com/fiber-laser-tube-cutting-parameters-guide.html
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://www.trumpf.com/en_US/products/software/programming-software/
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://4dsysco.com/wp-content/uploads/2025/10/Tecnomatix-Jack.pdf
- https://enterprise.trimech.com/uki/assembly-simulation-expert/
- https://edstechnologies.com/blog/understanding-delmia-virtual-simulation/
- https://arxiv.org/pdf/2508.18400
- https://www.haco.com/en/sheet-metal/software/hacobend-pro-3d
- https://almacam.com/?p=2878
- https://www.machineworks.com/sheet-metal-bending
- https://www.iitm.ac.in/ta/node/1058
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://wiki.freecad.org/Part_ShapeFromMesh/en
- https://grabcad.com/tutorials/how-to-convert-stl-to-step-using-freecad
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://hexagon.com/products/geomagic-design-x/geomagic-design-x-plans
- https://www.matterhackers.com/store/l/geomagic-design-x-software
- (fonte em procedencia.md)
- https://www.futuremarketinsights.com/reports/3d-reverse-engineering-software-market
- https://www.verifiedmarketresearch.com/product/reverse-engineering-software-market/
- https://github.com/3MFConsortium/lib3mf
- https://3mf.io/wp-content/uploads/sites/106/2025/02/3MF_Beam_Lattice_Extension_v1.2.0.pdf
- https://3mf.io/spec/
- https://github.com/elalish/manifold
- https://github.com/AcademySoftwareFoundation/openvdb
- https://github.com/libigl/libigl
- https://github.com/isl-org/Open3D
- https://www.cgal.org/license.html
- https://docs.ros.org/en/jazzy/p/ompl/__LICENSE.html
- https://dyn.manpages.debian.org/trixie/cura-engine/CuraEngine.1.en.html
- (fonte em procedencia.md)
- (fonte em procedencia.md)
- https://develop3d.com/3d-printing/dfm-analysis-for-3d-printed-parts-launched-by-protolabs/
- https://www.wevolver.com/article/design-for-additive-manufacturing-a-dfam-guide-for-engineers
- https://www.cadcrowd.com/blog/design-for-3d-printing-dfam-rules-for-fdm-sla-and-sls-parts-that-actually-work/
- https://www.businesswire.com/news/home/20250506950344/en/Additive-Manufacturing-Software-Markets-Report-2025-Analysis-Data-and-Forecast---Revenues-Expected-to-Hit-$6.78B-by-2033---ResearchAndMarkets.com
- https://www.grandviewresearch.com/industry-analysis/additive-manufacturing-market
- https://www.blmgroup.com/pt/lasertube
- https://br.messer-cutting.com/fileadmin/Portfolio_2026_-digital.pdf
- https://www.mfrural.com.br/busca/corte-tubos/estado/sao-paulo

STATUS: DONE_WITH_CONCERNS
SUMMARY: Relatório de benchmarking dos 13 itens, com mercado e proposta de ajustes ao backlog; o relatório saiu na resposta e não em arquivo, porque o modo de planejamento só permitiria gravar o arquivo de plano e você pediu para não criar arquivos, e faltam dados sobre preços de vários concorrentes e sobre o mercado de corte de tubos.
