# Benchmarking: tipos de cota e anotação do PMI

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

Este relatório não alterou nenhum arquivo do projeto. Só li código e fontes, e extraí um PDF público para o scratchpad. Marquei cada afirmação como **[V]**, confirmada em fonte lida nesta rodada, ou **[C]**, conhecimento geral não confirmado.

Limites da pesquisa, que afetam a confiança do que segue:
- As páginas `help.CAD avançado A.com` não renderizam sem JavaScript. O CAD avançado A vem de blogs de revendas e de trechos de busca.
- O guia oficial de FT&A do CAD avançado B (CAD avançado Bdoc.free.fr) recusou conexão. O CAD avançado B vem do guia de Drafting (via espelho), de relatos da IBM e de ementas de curso.
- Os docs do CAD avançado G redirecionam para a página de suporte, e a wiki do FreeCAD bloqueia leitura automática (Anubis). O blog do Fusion deu 403. Para esses três uso resultados de busca.
- Fonte forte: o **CAx-IF Recommended Practices for PMI (AP242) v4.1, 20/06/2024**, baixado e lido em texto. Também li os enums do OCCT (`XCAFDimTolObjects_*`) direto do GitHub.

---

## 1. Resumo e matriz tipo × software × Fenkra

### 1.1 Resumo

1. **O Fenkra cobre cerca de 12 tipos de anotação, e só o GD&T e a solda têm catálogo próprio.** Em `PmiKind` (`crates/fenkra-core/src/pmi.rs:694-726`) existem 11 variantes. Três delas são cotas de medida (`Linear`, `Radius`/`Diameter`, `Angle`). As outras são `Note`, `Roughness`, `Weld`, `Datum`, `Gdt`, `TitleBlock` e `Balloon`.
2. **Toda cota PMI mede a distância entre pontos soltos.** `Pmi::measure` (`pmi.rs:808-823`) calcula norma euclidiana entre `anchors`, e o diâmetro é `2·|centro−ponto|`. Não existe cota face–face, eixo–eixo, ao longo de eixo, de arco, de corda, de chanfro ou de espessura. Isso confirma o diagnóstico de `docs/pesquisa/pmi.md` (F1, F2) e o backlog B82.
3. **A tolerância é texto, não dado.** `tol_plus` e `tol_minus` são `String` (`pmi.rs:765-768`). O GD&T guarda valor e datums em `String` (`pmi.rs:715-721`) e tem **um** modificador textual (`GdtExtra.modifier`, `pmi.rs:352`). Isso impede exportar semântica para o AP242 e validar regras de norma.
4. **O AP242 distingue ao menos 28 tipos de cota** [V]. São 13 de localização e 15 de tamanho, mais as variantes `with path`. Os valores se expressam como nominal, qualificado min/max/médio, ±, faixa ou classe ISO 286. O Fenkra hoje só cobre `linear distance` (informal), `diameter`, `radius` e `angular`.
5. **Rugosidade, solda, chanfro e rosca não têm entidade semântica no AP242.** Procurei no RP v4.1 (texto extraído), e as seções de tolerância, datum e datum target cobrem cotas e GD&T. Esses itens viajam como apresentação gráfica (polilinha ou tesselada) e como texto de nota [V por ausência no documento]. Chanfro e rebaixo viram **várias cotas de tamanho independentes** (RP §5.1.9) [V].
6. **O OCCT 7.x já tem o vocabulário necessário.** `XCAFDimTolObjects_DimensionType` tem 32 valores, `DimensionModif` 24, `GeomToleranceModif` 17, `DatumSingleModif` 22 e `DatumTargetType` 5 [V]. O gargalo do Fenkra é o modelo de dados, não o kernel.
7. **O lado de entrada, o dos dados, já existe no Fenkra.** `Hole` (`crates/fenkra-core/src/doc.rs:2552-2584`) tem diâmetro, profundidade, tipo (rebaixado, escareado, contrafurado), rosca, ponta em V, conicidade e mão. Uma cota de furo completa não exige novo reconhecimento de feature, só ler o que já está no modelo. Isso é uma vantagem sobre o DimXpert, que precisa reconhecer features em sólidos sem histórico.
8. **Ordem recomendada:** alvos estáveis (B82, B83), depois tolerância tipada e tabelas normativas, depois o catálogo de cotas por par de alvos, depois cota de feature, depois FCF completo, datum targets e acabamento, e por fim AP242 semântico e esquema automático (seção 3).

**Legenda da matriz:** ✅ confirmado nesta rodada · ☑ provável, [C] · ➖ parcial ou indireto · ❌ não tem ou não achei · ❔ não verificado · **Desenho** indica que existe só no desenho 2D, não no PMI 3D. Colunas: CAT = CAD avançado B FT&A, CAD avançado A = CAD avançado A MBD/DimXpert, INV = CAD avançado C 3D annotations, CAD avançado F = CAD avançado F MBD + GD&T Advisor, CAD avançado G = CAD avançado G PMI, FUS = CAD avançado D, FC = FreeCAD TechDraw (2D, sem PMI 3D), FNK = Fenkra.

### 1.2 Matriz de cotas e anotações

**Cotas lineares**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Distância ponto–ponto | ✅ | ✅ | ✅ | ✅ | ☑ | ❔ | ✅ | `linear distance` | ✅ `pmi.rs:811` |
| Distância face–face (plano paralelo) | ✅ | ✅ Location | ✅ | ✅ só planos | ☑ | ❌ | ➖ | `dimensional_location` + `shape_aspect` das faces | ❌ |
| Distância aresta/eixo/ponto | ✅ | ✅ | ☑ | ➖ | ☑ | ❌ | ✅ | `linear distance` | ➖ meio de aresta |
| Horizontal / vertical / alinhada | ✅ (Drafting) | ➖ ref. | ☑ | ☑ | ☑ | ❌ | ✅ | `oriented` (eixo x do `axis2_placement_3d`) | **Desenho** `DimKind` (`fenkra-desenho/lib.rs:298`) |
| Entre centros (círculo–círculo) | ✅ | ✅ | ☑ | ✅ | ☑ | ❌ | ✅ | `linear distance` (padrão); 11 variantes centro/interior/exterior | ➖ só por clique |
| Mín/máx (arco a arco) | ☑ | ☑ | ❔ | ✅ 11 variantes | ☑ | ❌ | ❌ | `…centre/inner/outer × …` (Tabela 2) | ❌ |
| Em cadeia (chain) | ✅ stacked | ❔ | ❔ | ❔ | ✅ API `ChainDimension` | ❌ | ✅ | N/A (cada cota é independente) | ❌ |
| De base (baseline) | ✅ cumulated | ✅ DimXpert | ❔ | ❔ | ✅ API | ❌ | ✅ coordinate | idem | ❌ |
| Ordenada (coordenada) | ✅ | ☑ | ❔ | ❌ Advisor não avalia | ✅ API `OrdinateDimension` | ❌ | ✅ | idem | ❌ |
| Simétrica / meia cota | ✅ half dimensions | ☑ | ☑ | ❔ | ☑ | ❌ | ❌ | `…centre` com eixo | ❌ |
| Curvilínea (ao longo de caminho) | ✅ | ☑ | ❔ | ❌ arc length | ❌ | ❌ | ✅ arc length | `curved distance` / `…with path` | ❌ |
| Entre elemento e eixo da vista | ✅ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | `oriented` | ❌ |

**Angulares, radiais e diametrais**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Ângulo entre faces/arestas | ✅ | ✅ | ✅ | ➖ não avalia | ☑ | ❌ | ✅ | `angular_location` + `angle_selection` | ➖ 3 pontos `pmi.rs:814` |
| Complementar / suplementar / reflexo | ✅ | ✅ botão direito | ☑ | ❔ | ☑ | ❌ | ❌ | `.EQUAL./.LARGE./.SMALL.` | ❌ |
| Comprimento de arco | ✅ | ☑ | ☑ | ❌ | ☑ | ❌ | ✅ | `curve length` | ❌ |
| Corda | ❔ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | `linear distance` entre pontos | ❌ |
| Raio | ✅ | ✅ | ✅ | ☑ | ☑ | ❌ | ✅ | `radius` | ✅ `pmi.rs:697` (ponto) |
| Diâmetro | ✅ | ✅ | ✅ | ✅ | ☑ | ❌ | ✅ | `diameter` | ✅ `pmi.rs:699` (ponto) |
| Raio sacudido (jogged) | ❔ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | N/A (só apresentação) | ❌ |
| Raio controlado (CR) | ❔ | ❔ | ❔ | ❌ Advisor | ☑ | ❌ | ❌ | modificador `controlled radius` | ❌ |
| Diâmetro / raio esférico | ☑ | ☑ | ☑ | ❌ | ☑ | ❌ | ❌ | `spherical diameter/radius` | ❌ |
| Toroidal (menor/maior × diâm./raio) | ☑ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | 8 variantes `toroidal …` | ❌ |
| Raio de curvatura num ponto | ✅ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | N/A | ❌ |
| Raio verdadeiro (true radius) | ❔ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | N/A (é vista) | ❌ |

**Cotas de feature (chamadas)**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Furo simples (Ø e profundidade) | ✅ | ✅ Size | ✅ hole note | ✅ create | ☑ | ✅ | ➖ | 2 cotas (`diameter` + profundidade) | ❌ (existe `Hole` no modelo, não é lido) |
| Rebaixo (counterbore) | ✅ | ✅ | ✅ | ❌ depth | ☑ | ✅ | ❌ | **várias cotas** (§5.1.9) | ❌ |
| Escareado (countersink) | ✅ | ✅ Ø e ângulo | ✅ | ❔ | ☑ | ✅ | ❌ | + 1 cota angular | ❌ |
| Rosca | ✅ thread dim | ☑ | ✅ thread note | ❔ | ☑ | ✅ | ❌ | não há entidade; nota / apresentação | ❌ (existe `Hole.thread` e `Thread`, `doc.rs:2559`, `doc.rs:1345`) |
| Padrão "4× Ø6" | ✅ pattern | ✅ | ✅ `QTYNOTE` | ✅ ligado ao padrão | ☑ | ✅ | ✅ prefixo n× | tolerância/cota de padrão (§5.1.8, §6.4.1) | ➖ só `style.prefix` |
| Chanfro (C2, 2×45°) | ✅ 4 formatos | ✅ Size | ✅ chamfer note | ❌ Advisor | ✅ `Chamfer` API | ❔ | ✅ | **2 cotas** (comprimento + ângulo) | ❌ |
| Filete / arredondamento | ☑ | ✅ fillet radius | ☑ | ❌ round | ☑ | ❌ | ➖ | `radius` | ❌ |
| Rasgo / ranhura (slot) | ☑ | ✅ | ☑ | ✅ slotted | ☑ | ❌ | ❌ | `…with path` ou 2 cotas | ❌ |

**Espessura de parede, chapa, perfil**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Espessura (parede, chapa) | ☑ | ☑ | ☑ | ➖ tube sem suporte | ☑ | ❌ | ❌ | `thickness` | ❌ |
| Dobra / aba de chapa | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | N/A | ❌ (módulo Chapa não alimenta PMI) |
| Perfil estrutural | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | N/A | ❌ |

**Tolerâncias dimensionais**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| ± simétrica | ✅ | ✅ | ✅ | ✅ | ☑ | ❔ | ❌ | `plus_minus_tolerance` | ➖ texto (`pmi.rs:847`) |
| ± assimétrica / unilateral | ✅ | ✅ | ✅ | ✅ | ☑ | ❔ | ❌ | idem | ➖ texto (`pmi.rs:826`) |
| Limites (mín / máx) | ✅ | ✅ | ✅ | ☑ | ☑ | ❔ | ❌ | `value range` | ❌ |
| Classe ISO 286 (H7/g6) | ☑ | ☑ | ☑ | ☑ | ☑ | ❔ | ❌ | `limits_and_fits` (A–ZC / a–zc, IT01…IT18) | ❌ |
| Cota básica (TED) | ✅ framed | ✅ | ✅ | ➖ só obrigatória | ☑ | ❔ | ❌ | modificador `theoretical` | ➖ moldura `Caixa` |
| Cota de referência `(…)` | ☑ | ✅ reference dim | ☑ | ❔ | ☑ | ❔ | ❌ | modificador `auxiliary` | ➖ só `prefix` / `suffix` |
| Qualificador min / máx / média | ❔ | ❔ | ❔ | ❔ | ❔ | ❔ | ❌ | `…Qualifier_Min/Max/Avg` | ❌ |
| Inspecionável (balão oval) | ❔ | ❔ | ✅ inspection dimension | ❔ | ❔ | ❔ | ❌ | N/A (apresentação) | ❌ (`Balloon` existe, `PmiFrame::Arredondada` serve de oval) |
| Não em escala | ❔ | ❔ | ❔ | ❔ | ❔ | ❔ | ❌ | N/A | ❌ |
| Modificadores ISO 14405-1 (LP, GG…) | ❌ | ✅ símbolos 2023 | ❔ | ❌ | ☑ | ❔ | ❌ | `DimensionModif` (24 valores) | ❌ |
| Contínua CF, estatística, envelope | ✅ | ✅ | ✅ | ❔ | ☑ | ❔ | ❌ | `continuous feature`, `statistical`… | ❌ |

**GD&T, datums e alvos**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| 14 características ISO 1101 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | 15 entidades de tolerância | ✅ `pmi.rs:11-26` |
| 12 características ASME 2018 (sem concentricidade e simetria) | ☑ | ☑ | ☑ | ✅ | ☑ | ❌ | ❌ | tabela 12 ainda inclui as removidas | ❌ (o Fenkra oferece as 14 sempre) |
| Advisor (só o que vale para o elemento) | ✅ | ➖ | ✅ | ✅ | ✅ PMI Advisor | ❌ | ❌ | N/A | ❌ |
| Zona Ø / esférica / largura | ✅ | ✅ | ✅ | ✅ | ☑ | ❌ | ❌ | `tolerance_zone_form` (17 nomes) | ➖ só Ø (`zone_dia`) |
| MMC / LMC / RFS | ✅ | ✅ | ✅ | ✅ | ☑ | ❌ | ❌ | `…_MATERIAL_REQUIREMENT` | ➖ Ⓜ Ⓛ por texto (`pmi_ui.rs:1073`) |
| Projetada, livre, tangente | ✅ | ✅ | ✅ | ✅ | ☑ | ❌ | ❌ | `…with value (projected)`, `free state`, `tangent plane` | ➖ Ⓟ Ⓕ Ⓣ por texto |
| CZ, ACS, UF, LE, NC, ST | ☑ | ☑ | ☑ | ✅ Details | ☑ | ❌ | ❌ | `COMMON_ZONE`, `ANY_CROSS_SECTION`, `UNITED_FEATURE`… | ❌ |
| Desigual (Ⓤ), máx., por unidade | ☑ | ☑ | ☑ | ❌ per-unit | ☑ | ❌ | ❌ | `unequally_disposed…`, `…with_maximum`, `unit basis` | ➖ Ⓤ por texto |
| Todo o entorno / todo o contorno | ☑ | ☑ | ✅ All Around / All Over | ☑ | ☑ | ❌ | ❌ | `ALL_AROUND`, `ALL_OVER` | ❌ |
| Entre A↔B (between) | ☑ | ☑ | ☑ | ☑ | ☑ | ❌ | ❌ | `between_shape_aspect` | ❌ |
| Quadro composto / segmento único | ✅ | ✅ | ✅ | ☑ | ☑ | ❌ | ❌ | §6.9.9 | ➖ 2 linhas só texto (`pmi.rs:355`) |
| Perfil dinâmico (ASME 2018) | ❔ | ❔ | ❔ | ✅ Details | ❔ | ❌ | ❌ | **sem enum no OCCT** | ❌ |
| Datum simples | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | `datum` + `datum_feature` | ➖ letra solta (`pmi.rs:713`) |
| DRF (A\|B\|C) | ✅ | ✅ | ✅ DRF | ✅ | ✅ | ❌ | ❌ | `datum_system` | ❌ (string) |
| Datum comum (A-B) | ✅ | ☑ | ☑ | ❌ 3 datums | ☑ | ❌ | ❌ | compartimento comum | ❌ |
| Datum target (ponto/linha/área) | ✅ | ❔ | ✅ Point, Circle, Rect | ❌ PNT | ✅ (Advisor desde 2212) | ❌ | ❌ | `placed_datum_target_feature` + 5 tipos | ❌ |
| Alvo móvel (movable) | ☑ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | §6.6.4 | ❌ |
| Áreas restritas | ✅ | ❔ | ❔ | ❔ | ❔ | ❌ | ❌ | `datum_target` tipo `area` | ❌ |

**Acabamento, solda, notas e tabelas**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Rugosidade (símbolo básico + Ra) | ✅ | ✅ | ✅ | ☑ | ✅ | ❌ | ☑ | sem entidade; apresentação | ✅ `pmi.rs:704` |
| ISO 21920 / ISO 1302 / texturas | ☑ | ✅ ISO 21920 (2025 SP1) | ☑ | ☑ | ☑ | ❌ | ❌ | idem | ❌ |
| Solda ISO 2553 / AWS A2.4 | ☑ | ✅ | ☑ | ☑ | ✅ | ❌ | ☑ | sem entidade; apresentação | ✅ ISO 2553 sistema A (`pmi.rs:86-342`); sem AWS |
| Nota livre / com chamada | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ leader note | ☑ | `descriptive_representation_item` + gráfico | ✅ `pmi.rs:702` |
| Nota com parâmetros `{prop}` | ☑ | ✅ | ✅ | ☑ | ☑ | ❌ | ❌ | N/A | ➖ só na legenda 3D (`pmi.rs:723`) |
| Flag note | ✅ | ☑ | ❔ | ❔ | ❔ | ❌ | ❌ | N/A | ➖ moldura bandeira |
| Balão | ❔ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | N/A | ✅ `pmi.rs:725` |
| Nota de área / região | ✅ restricted | ❔ | ✅ general profile note | ✅ | ☑ | ❌ | ❌ | `ALL_OVER` / zona | ❌ |
| Nota de perfil geral | ☑ | ☑ | ✅ | ✅ | ☑ | ❌ | ❌ | tolerância "all over" (§6.3) | ❌ |
| Tabela 3D | ✅ hole table | ✅ BOM | ✅ | ❔ | ❔ | ❌ | ✅ | N/A | ❌ |

**Auto cotagem**

| Tipo | CAT | CAD avançado A | INV | CAD avançado F | CAD avançado G | FUS | FC | AP242 / OCCT | FNK |
|---|---|---|---|---|---|---|---|---|---|
| Auto cotagem | ➖ | ✅ Auto Dimension Scheme | ❌ | ✅ Create Selected | ✅ MBD logical rules | ❌ | ❌ | N/A | ❌ |
| Verificação sub/super restringido | ✅ Check Report | ✅ Show Tolerance Status | ✅ Advisor + Face Status | ✅ Constraint State | ✅ PMI Advisor | ❌ | ❌ | N/A | ❌ (B38) |
| Cotas a partir de features | ➖ | ✅ DimXpert | ✅ promote | ✅ | ☑ | ✅ hole notes | ❌ | N/A | ❌ |

---

## 2. Fichas por grupo de tipos

Cada ficha traz: **como funciona nos melhores**, **o que o Fenkra tem**, **regras que evitam erro** e **saída AP242**.

### G1. Cotas lineares por par de alvos

**Como funciona no melhor.**
- **CAD avançado A:** o DimXpert tem duas famílias. O *Size* mede o tamanho de uma feature reconhecida (furo, rasgo, chanfro). O *Location* mede entre **duas features DimXpert**, em linear ou angular, e **exclui** superfície, filete, chanfro e bolsa [V]. O desvio é intencional: uma cota de localização só tem sentido entre elementos de posição bem definida.
- **CAD avançado A:** a *Reference dimension* (Smart Dimension na barra MBD) mede entre arestas, linhas e curvas sem reconhecer feature, e por isso é só informativa [V].
- **CAD avançado B:** a cota semântica nasce no *Tolerancing Advisor* com *Limit of Size Definition* e condição *Enveloping*. A cota **enquadrada** (framed) só é aceita se estiver ligada a referência parcial, área restrita ou tolerância [V, de `docs/pesquisa/pmi.md`].
- **CAD avançado F:** a cota de tamanho só é avaliada **entre dois planos paralelos**; plano–cilindro e plano–esfera ficam sem suporte no Advisor, mas existem no CAD avançado F nativo [V].
- **CAD avançado G:** a API tem builders separados para `ChainDimension`, `BaselineDimension` e `OrdinateDimension` (esta com modo *single* e *multiple*) [V].
- **FreeCAD TechDraw 1.0:** *chain* (horizontal, vertical e oblíqua), *coordinate* (cotas igualmente espaçadas partindo de uma base comum) e *extent* [V, resultado de busca].

**Passos de criação (consenso):**
1. Selecione dois elementos (face, aresta, ponto, eixo ou centro).
2. O programa deduz o modo (distância mínima, ao longo de eixo X/Y/Z, projetada no plano de anotação).
3. Posicione o texto no plano de anotação, arrastando a linha de cota.
4. Edite precisão, tolerância e prefixo/sufixo.

**Fenkra hoje.**
- `PmiKind::Linear` mede a norma entre duas âncoras (`pmi.rs:811`).
- `anchors()` guarda o ponto médio da aresta ou o ponto clicado na face (`pmi_ui.rs:53-58`).
- `snap` reencaixa em até 2 mm (`pmi_ui.rs:243-255`).
- No desenho 2D existem as variantes `Horizontal`, `Vertical` e `Aligned` (`fenkra-desenho/src/lib.rs:298-305`), com projeção em `layout.rs:675-690`. O PMI 3D **não** reaproveita isso.
- O valor mostra duas casas decimais com zeros cortados e vírgula fixa (`pmi.rs:840-844`).

**Regras que evitam erro:**
- A medida vem do **par de alvos resolvido**, nunca de pontos clicados. Face–face plana paralela mede a distância perpendicular; se as faces deixarem de ser paralelas, a cota passa a status "Ambígua", não a um número diferente.
- O modo "entre centros" usa o eixo da feature, não o centro de uma aresta.
- Para círculos o AP242 distingue 11 variantes (centro/interior/exterior) [V]. A UI deve oferecer "de centro a exterior" etc. como seleção explícita; o padrão é centro a centro.
- Cadeia, base e ordenada são **conjuntos** com origem comum. Mudar a origem recalcula todo o conjunto. O CAD avançado G avisa quando o empilhamento de ordenada com cadeia não é permitido [V].
- Em cadeia, a soma dos parciais deve bater com a cota total, senão há sobrecota (aviso, não bloqueio).

**Saída AP242.**
- `dimensional_location` com `name` = `linear distance` (padrão) ou uma das variantes da Tabela 2 (`linear distance centre outer`, etc.) [V].
- `curved distance` para curvilíneas.
- `directed_dimensional_location` quando relating→related importa.
- `dimensional_location_with_path` quando há caminho.
- No OCCT: `XCAFDimTolObjects_DimensionType_Location_LinearDistance` e suas 8 variantes `FromCenterToOuter…`, `Location_CurvedDistance`, `Location_Oriented`, `Location_WithPath` [V].
- Chain, baseline e ordinate **não têm entidade própria**. Cada cota sai independente; o agrupamento é só gráfico (`draughting_callout_relationship`, RP §9.3.3.1) [V].

### G2. Cotas angulares, de arco e corda

**Como funciona no melhor.**
- **CAD avançado B** (Drafting): cota angular com mudança de setor e opção "suplementar" [V]. Cota de comprimento curvilíneo é a mesma cota de distância, com o menu de contexto trocando *Overall* por *Curvilinear Length* e o símbolo de semi-arco sobre o valor [V].
- **CAD avançado A:** a cota de referência angular entre arestas pode mostrar o suplemento conforme a posição do cursor; botão direito trava o valor e alterna [V].
- **CAD avançado F:** ângulo pode aparecer no esquema mas **não é avaliado** pelo Advisor; arc length e ordinate também não [V].
- **FreeCAD:** ângulo entre duas arestas retas; *3-point angle*; *arc length* para arestas selecionadas [V].
- **Cone:** o DimXpert reconhece o cone e mostra o ângulo; para meio-ângulo precisa customizar [V].

**Fenkra hoje.** `PmiKind::Angle` com três âncoras, `acos` do ângulo entre vetores, faixa de 0° a 180° (`pmi.rs:814-820`). O desenho do arco está em `pmi_ui.rs`. No desenho 2D, `DimKind::Angle` (`lib.rs:304`) tem a mesma limitação. Não existe arco, corda nem ângulo reflexo.

**Regras que evitam erro:**
- Ângulo entre **duas faces planas** vem das normais, não de pontos. Entre aresta e face, é 90° menos o ângulo entre a aresta e a normal.
- Quatro modos explícitos: interno, externo, complementar e reflexo. Mostrar o modo no painel para o usuário não depender da posição do cursor.
- Comprimento de arco lê o comprimento da curva da aresta, não a soma de cordas da malha (a malha subestima).
- Corda é distância reta entre os extremos do arco: reaproveita G1.
- Arestas que não se cruzam e não são coplanares retornam erro "ângulo indefinido".

**Saída AP242.**
- `angular_location` (entre duas features) e `angular_size` (numa só), com `angle_selection` = `.EQUAL.`, `.LARGE.` ou `.SMALL.`; acima de 180° é o ângulo reflexo [V].
- Arco = `dimensional_size` `curve length`.
- OCCT: `Location_Angular`, `Size_Angular`, `Size_CurveLength` [V].
- Corda sai como `linear distance` entre dois pontos de aresta.

### G3. Raio, diâmetro e variantes

**Como funciona no melhor.**
- O raio ou diâmetro vem da **superfície analítica** (cilindro, cone, esfera, toro), não de pontos. O CAD avançado B mostra o fluxo: selecionar o furo, abrir o Advisor, escolher *Diameter*, informar tolerância e *Enveloping* [V, de `pmi.md`].
- A esfera tem cota própria (`spherical diameter/radius`), e o toro tem 8 variantes (menor/maior, raio/diâmetro, e os limites alto/baixo) [V].
- O modificador *controlled radius* (CR) é **prefixo**, ao contrário dos demais, que são sufixos [V].

**Fenkra hoje.** `Radius` e `Diameter` usam centro ajustado por `fit_circle` mais um ponto da aresta (`pmi_ui.rs:61-67`). O valor é distância de pontos. Raio sacudido (jogged), diâmetro de esfera e raio verdadeiro não existem. O desenho de diâmetro cruza o centro (`pmi_ui.rs:386-392`).

**Regras que evitam erro:**
- O valor vem do raio da face cilíndrica, cônica, esférica ou tórica. Se a face mudar de tipo (virar B-spline), a cota fica "Isolada", não mede ponto a ponto.
- O prefixo (Ø, R, SØ, SR) deriva do tipo de cota, não do texto livre.
- Raio sacudido é só apresentação: o usuário move o ponto de partida da cota do centro verdadeiro para um centro fictício. Guarde o deslocamento, não um novo valor.
- "Raio verdadeiro" em vista distorcida pertence à vista de anotação (planos), não ao tipo de cota.

**Saída AP242.** `dimensional_size` com `name` = `diameter`, `radius`, `spherical diameter`, `spherical radius` ou um dos oito `toroidal …` [V]. OCCT: `Size_Diameter`, `Size_Radius`, `Size_SphericalDiameter`, `Size_SphericalRadius`, `Size_Toroidal*` [V]. CR sai como modificador `controlled radius` (type 2, §5.3) [V].

### G4. Cotas de furo, rosca, chanfro e filete

**Como funciona no melhor.**
- **CAD avançado C:** *Hole/Thread Note* e *Chamfer Note* são comandos separados. O 2019 mostra a **quantidade** do padrão, e `QTYNOTE` liga o realce cruzado aos furos do padrão [V]. O 2027 permite anexar o identificador de datum a notas de furo e de rosca [V].
- **Fusion:** *Hole/Thread Notes* e *Leader Notes* são o PMI nativo, com ASME e ISO [V, resultado de busca].
- **CAD avançado A:** o *Size* do DimXpert lê furo (Ø, profundidade), escareado (Ø e ângulo), rasgo (comprimento e largura), filete (raio) e chanfro. O chanfro tem dois esquemas: **distância × ângulo** e **distância × distância** [V]. Clicar na face do chanfro basta para reconhecê-lo [V].
- **CAD avançado B:** *Chamfer Dimension* oferece quatro formatos (comprimento × comprimento, comprimento × ângulo, ângulo × comprimento, só comprimento), por seleção ou detecção automática [V]. Uma ocorrência da IBM mostra a limitação em curvas: só funciona com plano/plano/plano, plano/cone/cilindro, plano/cone/cone e cilindro/cone/cilindro [V].
- **CAD avançado F:** cria automaticamente as cotas de furos simples, eixos, rasgos, placas e furos oblongos. Chanfro, arredondamento e tubo **não são avaliados** [V]. Ao escolher um furo de padrão CAD, pergunta "padrão inteiro?" e mantém a ligação [V, de `pmi.md`].
- **FreeCAD:** chanfro horizontal/vertical (tamanho e ângulo) e prefixo `n×` [V].
- **Caso composto:** a ambiguidade de qual diâmetro recebe a tolerância num furo com rebaixo só se resolve **quebrando a chamada** [V].

**Fenkra hoje.** Não existe cota de feature no PMI. Mas `Hole` (`doc.rs:2552-2584`) guarda tudo o que a chamada precisa:
- `kind` com `Simples`, `Rebaixado`, `Escareado` e `Contrafurado` (`doc.rs:2543-2549`);
- `diameter`, `depth`, `thread`, `thread_modeled`, `tip`, `taper` e `left_hand`;
- `points` e `sketch` para a contagem do padrão.

Chanfro e filete existem como features, mas o PMI não as lê. O backlog B107 prevê "chamadas de furo e rosca lidas da feature, com contagem 4×" **só no desenho**.

**Regras que evitam erro:**
- A chamada é **gerada** do `Hole`/`Thread`/`Chamfer`/`Fillet`: `4× Ø6,5 ▽12 · M8×1,25-6H`. Editar o furo atualiza a nota; o usuário não digita o texto.
- Chamada composta sempre pode ser **quebrada** em cotas simples (Ø, profundidade, ângulo). Cada cota recebe tolerância independente. É o que o AP242 faz de qualquer jeito.
- Quantidade (`n×`) vem da contagem do padrão. Se o padrão mudar de 4 para 6, a nota acompanha; se um furo sair do padrão, status "Ambígua".
- Rosca: o diâmetro nominal vem da série (M8), o passo da tabela, a classe de tolerância (6H/6g) do campo `standard`. Não calcule o diâmetro de fundo no texto.
- Chanfro simétrico (45°) tem formato `C2` ou `2×45°`; assimétrico exige `a×b` ou `a×ângulo`.

**Saída AP242.** Não há entidade para rosca, chanfro nem chamada composta. O RP manda emitir **várias `dimensional_size`/`dimensional_location` independentes** (rebaixo = Ø do rebaixo + profundidade; escareado + ângulo; sem relação entre elas) [V]. A nota da rosca vai como texto + gráfico. O chanfro vira `linear distance` + `angular`. Quantidade vai pelo padrão (RP §5.1.8, §6.4.1) [V].

### G5. Espessura, chapa e perfil estrutural

**Como funciona no melhor.** O AP242 tem `thickness` como `dimensional_size` [V], e o OCCT `Size_Thickness` [V]. O CAD avançado F cita a espessura de parede entre cilindros coaxiais como exemplo de caso que o Advisor **não** avalia (tubo) [V]. Dobra e aba de chapa, e perfil estrutural, não aparecem em nenhum dos softwares como tipo de cota nativo nas fontes lidas; chapa é cotada por raio, ângulo e comprimento planificado.

**Fenkra hoje.** Nada. Mas o Fenkra tem módulos de chapa e estrutural cuja geometria paramétrica sabe a espessura, o raio de dobra, o K-factor e o perfil. Esse é um espaço onde o Fenkra pode **superar** os concorrentes.

**Regras que evitam erro:**
- Espessura lê o parâmetro da feature de chapa ou mede entre duas faces paralelas opostas; em parede curva, entre superfícies equidistantes (offset constante), com tolerância de aviso se não forem.
- Dobra: cota de raio interno, ângulo e comprimento da aba, lidas da feature.
- Perfil estrutural: nota gerada do perfil da biblioteca (`HEA 200 – L=2400`), campos `{perfil}` e `{comprimento}`.

**Saída AP242.** `thickness` (espessura); raio e ângulo de dobra como `radius` e `angular_size`; perfil como nota e propriedade (não há entidade) [V quanto à lista do AP242; o resto é recomendação].

### G6. Tolerâncias dimensionais, ajustes e cotas básicas

**Como funciona no melhor.**
- **AP242 define o contrato** [V]: valor nominal; nominal + qualificador (`min`/`max`/`avg`); nominal + limites ±; faixa de valores; **classe de tolerância** em `limits_and_fits`, com `form_variance` (A–ZC para furo, a–zc para eixo), `zone_variance` (`hole`/`shaft`) e `grade` (um dos 18 graus IT da ISO 286).
- **Modificadores tipo 1:** `theoretical` (básica, ASME; teórica, ISO) e `auxiliary` (referência) são **mutuamente exclusivos** [V]. Tipo 2 são múltiplos: CR, quadrado, estatística, CF, tamanho por dois pontos, tamanho por esfera, critérios de associação (mínimos quadrados, inscrito máximo, circunscrito mínimo), diâmetros calculados (circunferência, área, volume), tamanhos por ordem (máx., mín., média, mediana, ponto médio, faixa), qualquer porção, qualquer seção, seção fixa, tolerância comum, estado livre, UF [V].
- **CAD avançado A:** tipos *Plus and Minus* e *Geometric*; o esquema geométrico troca ± por posição e cria cotas básicas; *Recreate basic dim* repara o esquema [V]. MBD 2023 adicionou símbolos ISO 14405-1; MBD 2024, separador decimal e cotas duplas [V].
- **CAD avançado B:** a cota enquadrada só vive ligada a referência parcial, área restrita ou tolerância [V]. A cota angular mostra limites superior e inferior em *Limit of Size* [V].
- **CAD avançado C:** inclui *Inspection dimension*, valor oculto e CF. A cota promovida do modelo permanece ligada ao parâmetro [V].

**Fenkra hoje.** `tol_plus` e `tol_minus` são `String` livres, e `display` monta `±x` quando iguais, ou `+a/−b` (`pmi.rs:845-851`); `display_parts` empilha as assimétricas (`pmi.rs:826-836`). Não valida número, não tem limites, ISO 286, básica, referência, nem precisão por norma. A vírgula decimal é fixa (`pmi.rs:842`). No desenho, `Dim.tol` também é `String` (`fenkra-desenho/src/lib.rs:333`). B108 já prevê o tipo e a tabela ISO 286, mas ainda sem dono para a **tabela normativa**.

**Regras que evitam erro:**
- Tipo `Tolerance` fechado: `Nenhuma | Simetrica(v) | Bilateral(+a,−b) | Limites(min,max) | Classe{furo|eixo, letra, IT}`, mais um estado ortogonal `basica | referencia | inspecionavel`.
- A classe ISO 286 resolve desvios por **tabela por faixa de diâmetro nominal** (ex.: Ø20 H7 = +0,021/0). A tabela é dado versionado, com teste contra valores publicados.
- Cota básica sem quadro de posição/perfil/angularidade que a referencie gera **aviso** ("cota básica órfã"). Cota de referência não entra no empilhamento (B126).
- Precisão de tolerância e de valor seguem o mesmo número de casas na norma ASME [C]; a precisão vem da norma ativa, não do código.
- Tolerância geral (ISO 2768-1/-2) vai na nota geral e vale para cotas sem tolerância individual [C].
- Rejeitar: tolerância ≥ valor, sinal incoerente, texto não numérico.

**Saída AP242.** `plus_minus_tolerance` (±), `tolerance_value`/`value_range` (limites), `limits_and_fits` (classe), `qualified_representation_item` (casas decimais, §5.4), modificadores tipo 1/tipo 2 em `descriptive_representation_item` (§5.3) [V]. OCCT: `XCAFDimTolObjects_DimensionObject` com `SetUpperBound`/`SetLowerBound`/`SetClassOfTolerance`, qualificador `Min/Max/Avg` e os 24 `DimensionModif` [V para os enums; nomes dos setters [C]].

### G7. Notas, balões, tabelas e flag notes

**Como funciona no melhor.**
- **CAD avançado G:** a nota tem "manter paralela à tela" e "bloquear tamanho e posição" [V].
- **CAD avançado C:** nota com chamada 3D permite **várias** faces, arestas e vértices associados à mesma chamada (*Add/Remove Associated Geometry*), parâmetros no texto e borda retangular [V].
- **CAD avançado B:** *flag note* e texto com chamada fazem parte de FT&A; tabela de furos e de coordenadas de pontos vêm do Drafting [V].
- **Nota de perfil geral** (CAD avançado C, CAD avançado F): "superfícies não anotadas: perfil 0,3 em relação a A|B|C" [V].

**Fenkra hoje.** `Note`, `Balloon` e `TitleBlock` com `{campos}` (`pmi.rs:702`, `723`, `725`, `800`). Chamada de uma seta só (B23 prevê várias). Não há flag note como tipo (existe a moldura `BandeiraDir/Esq`, `pmi.rs:371-372`), nota numerada, tabela 3D nem nota de região.

**Regras que evitam erro:**
- Campos `{prop}` em **qualquer** nota, resolvidos na hora de desenhar e exportar.
- Nota geral numerada ("NOTAS: 1. …") fica numa área própria, com renumeração automática.
- Apagar um alvo da chamada deixa a nota com as setas restantes e aviso, nunca apaga a nota.

**Saída AP242.** Nota como `descriptive_representation_item`/texto editável (RP §7.4, texto semântico com propriedades de validação) mais a apresentação gráfica [V]. Flag note, balão e tabela são só gráficos.

### G8. GD&T completo: símbolos, modificadores e quadros

**Como funciona no melhor.**
- **Advisor:** CAD avançado B mostra só as características válidas para o elemento selecionado, e a opção semântica é obrigatória para isso [V]. O CAD avançado F filtra símbolos pela feature e tem aba *Details* com modificadores para ISO 1101:2017 e ASME Y14.5-2018 [V]. O CAD avançado G tem *PMI Advisor* com checagens personalizadas [V].
- **CAD avançado C 2027:** o painel de FCF tem quatro tipos de seleção (**Integral Feature, Derived Feature, All Around, All Over**), de 1 a 3 especificações por quadro (Forma, Orientação, Posição, Batimento), nós superior e inferior, e composição visual [V].
- **CAD avançado A:** arrasta o quadro sobre uma chamada de cota e os dois se fundem [V].
- **ASME Y14.5-2018** removeu concentricidade e simetria; trouxe **perfil dinâmico**; renomeou "dual datum" para *Common Datum Features* e "datum feature simulator" para *True Geometric Counterpart*; CF dispensa o `nX` [V]. O ISO 1101 continua com 14.

**Fenkra hoje.**
- 14 características com símbolos Unicode (`pmi.rs:29-63`).
- `GdtExtra` (`pmi.rs:346-356`): `zone_dia`, **um** `modifier: String` e `second: Option<(String,String)>` para a linha do quadro composto.
- Valor e datums são `String` (`pmi.rs:715-721`); a UI é `gdt_ui` (`pmi_ui.rs:1055-1094`). Modificadores oferecidos: Ⓜ Ⓛ Ⓟ Ⓕ Ⓢ Ⓣ Ⓤ.
- O símbolo é um **glifo de fonte** (`pmi.rs:46-63`). O desenho do quadro está em `pmi_draw.rs:473`.
- O desenho 2D replica o mesmo modelo em `DwgGdt` (`fenkra-desenho/src/lib.rs:371-377`).

**O que falta (lista direta):**
1. **Matriz de validade por elemento e norma.** Forma: sem datum; orientação, posição, batimento: com datum; perfil: com ou sem. ASME 2018 desativa concentricidade e simetria.
2. **Zona:** largura (padrão), Ø, SØ, entre dois círculos concêntricos, não uniforme, zona projetada **com valor**. O AP242 lista 17 formas de zona [V]. Hoje só há Ø ligado/desligado.
3. **Modificadores como enum**, com compatibilidade por característica e posição: do valor (Ⓜ Ⓛ Ⓟ Ⓕ Ⓣ Ⓤ), do datum (Ⓜ Ⓛ), do quadro (CZ, ACS, UF, ↔ between, ⌓ all around, all over, CF, `n×`, `LE`, `NC`, `ST`). O AP242 lista 30+ para tolerância [V] e o OCCT 17 [V]. O Fenkra tem 7 por texto.
4. **Quadro composto** (PLTZF/FRTZF) e **segmento único** com cada linha tendo tolerância, modificadores e datums próprios.
5. **Tolerância por unidade** (0,05/25), **valor máximo** (`0,1 MAX`), **desigual** (Ⓤ com deslocamento).
6. **Perfil dinâmico** (ASME 2018), sem enum no OCCT [V]: só sai gráfico.
7. **Símbolos vetoriais** (hoje glifos de fonte), que geram "tofu" se a fonte não tiver `⏤ ⏥ ⌭ ⌖ ⌯ ⌰ ⌓` [C] e não exportam bem para DXF/PDF. A solda já faz isso (`weld_symbol`, `pmi.rs:300`).
8. Coaxialidade e concentricidade são entidades separadas no AP242 (Tabela 12 do RP) [V]. A distinção de uso entre as duas na ISO 1101 é [C].
9. A dica do modificador Ⓢ ("independente", `pmi_ui.rs`, texto de ajuda) não corresponde a símbolo da ASME 2018 nem da ISO 1101 (independência da ISO 8015 usa Ⓘ) [C]. Revisar.

**Regras que evitam erro** (mensagens do CAD avançado C Advisor, que servem de lista de regras) [V]:
- "Tolerância de forma deve ser menor que a de tamanho."
- "Tolerância no segundo nível deve ser menor que a do nível anterior" (composto).
- "Zona projetada deve ser maior que zero."
- "Valor zero (ou inconsistente) não permitido."
- "Quadros compartilham faces."
- "Posição de padrão exige DRF completo."
- Ⓜ/Ⓛ só em feature de tamanho; Ⓟ só em posição/orientação; Ⓕ só em forma.

**Saída AP242.**
- Uma entidade por característica: `flatness_tolerance`, `position_tolerance`, `surface_profile_tolerance`… (Tabela 12), com 0 a 3 datums [V].
- Modificadores como entidade complexa `geometric_tolerance_with_modifiers` [V].
- Zona em `tolerance_zone` + `tolerance_zone_form` (nome = forma) [V].
- Desigual em `unequally_disposed_geometric_tolerance`, máximo em `geometric_tolerance_with_maximum_tolerance`, unidade em `unit-basis` [V].
- Composto em §6.9.9 [V].
- OCCT: `GeomToleranceType` (16), `GeomToleranceTypeValue` (Ø, SØ), `GeomToleranceModif` (17) e `DatumModifWithValue` (circular, distância, projetada, esférica) [V].

### G9. Datums, DRF e alvos de referência

**Como funciona no melhor.**
- **CAD avançado B:** o datum simples nasce selecionando a face e escolhendo o ícone no Advisor. O **sistema** (A|B|C) exige datums prévios; adiciona primário, secundário e terciário em ordem [V]. Datum comum (A-B) e **datum targets**/áreas restritas têm tópicos próprios [V].
- **CAD avançado C:** *DRF* com primário/secundário/terciário e modificadores, editável com *Edit DRF*; *Datum Target* com Ponto, Círculo e Retângulo (**tipo imutável depois de criado**), também em montagem e soldagem desde o 2027 [V]. Só **um** identificador por face plana [V]. Mensagens: "DRF não referenciado", "Especificação do DRF incompleta", "Orientação do datum secundário não permitida", "Referência do alvo ausente" [V].
- **CAD avançado F:** monta o **DRF predominante**, sugere as primeiras features e exige restringir os 6 graus de liberdade (exceto peças simétricas). Co-datum com três features e datum PNT (esfera/soquete) **não** são suportados no Advisor [V].
- **CAD avançado G:** o Advisor só passou a aceitar datum target como referência na 2212 [V].

**Fenkra hoje.** `Datum { letter: String }` na âncora clicada (`pmi.rs:713`), símbolo em `pmi_draw.rs:518`. O quadro GD&T guarda os datums como texto ("A B C", `pmi.rs:865`), **sem vínculo** com a anotação `Datum`. Letras duplicadas são aceitas; apagar um `Datum` deixa o quadro apontando para uma letra inexistente. Sem datum target, DRF, datum comum nem tipo de datum.

**Regras que evitam erro:**
- Letra única por peça; proibidas I, O, Q [C]; renomear atualiza todos os quadros que a referenciam (referência por ID).
- Secundário deve restringir graus de liberdade ainda livres. Orientação exige ao menos o primário; posição de padrão exige DRF completo.
- Datum comum (A-B) é objeto próprio com dois alvos.
- Datum target: tipo (ponto, linha, círculo, círculo-curva, retângulo, área, curva), número (`A1`), e dimensões do alvo. O número de alvos por datum segue a regra 3-2-1 (aviso, não bloqueio) [C].
- Alvo móvel (movable) carrega direção e curso (RP §6.6.4) [V].

**Saída AP242.** `datum`, `datum_feature`, `datum_system` (lista ordenada de `datum_reference_compartment`, com modificadores), `datum_reference_element` para o comum; alvos: `placed_datum_target_feature` com `target_id` só numérico (a letra fica no `datum.identification`) e `shape_representation_with_parameters` com `target diameter`/`target length`/`target width` para ponto, linha, círculo, círculo-curva e retângulo; área e curva são alvos **explícitos** com `advanced_face`/curva [V]. OCCT: `DatumTargetType` (Point, Line, Rectangle, Circle, Area) e `DatumSingleModif` com restrições de grau de liberdade U/V/W/X/Y/Z [V].

### G10. Acabamento superficial e solda

**Rugosidade.**
- **Estado das normas:** a ISO 21920-1 (EN 2022) substitui a ISO 1302; traz novos elementos de especificação, e a regra de aceitação padrão passou a ser a do **máximo** [V]. O CAD avançado A 2025 SP1 suporta ISO 21920 além de ISO 1302:2002 e 1992 [V]. O símbolo com barra para a nova norma [C, só citado em fonte secundária].
- **O que o AP242 faz:** não há entidade de textura no RP v4.1 [V por ausência]. Segundo `pmi.md`, o CAD Exchanger usa `SurfaceFinishComponent` com textura, método e sobremetal [V de pesquisa anterior]; o OCCT/XCAF não tem equivalente [V por ausência de enum].
- **Fenkra:** `Roughness { ra, process }` (`pmi.rs:704`), desenhado como "√ Ra x (processo)". Faltam: símbolo "remoção obrigatória/proibida", outros parâmetros (Rz, Rmax, Rq), limite superior/inferior, comprimento de amostragem, direção de lay, sobremetal, "todo o contorno", norma e textura de área.
- **Regra:** a rugosidade é **atributo da face**, não do ponto. A âncora referencia a face; o valor viaja como propriedade quando houver exportação.

**Solda.**
- **Fenkra tem a base mais completa do conjunto:** 11 tipos ISO 2553 (`WeldKind`, `pmi.rs:86-101`), dados completos do sistema A com linha tracejada, lado oposto, comprimento `n×l (e)`, bandeira de campo, contorno (plano, convexo, côncavo), contorno ao redor e processo ISO 4063 na cauda (`WeldExtra`, `pmi.rs:161-179`), desenhados por primitivas vetoriais (`weld_symbol`, `pmi.rs:300`).
- **Lacunas:** sistema B (linha de referência dupla), padrão AWS A2.4 (símbolos suplementares, "backing", "melt-thru", "flare-bevel/flare-V", ponto/costura com dimensões AWS) [C], e geração a partir de cordão estrutural.
- **AP242:** sem entidade semântica; só apresentação [V por ausência].

**Saída AP242 resumida:** ambos viajam como PMI gráfico (polilinha ou tesselado) mais propriedade textual; não esperar semântica máquina-a-máquina.

### G11. Auto cotagem e verificação do esquema

**Como funciona no melhor.**
- **CAD avançado A Auto Dimension Scheme:** escolher tipo de peça, tipo de tolerância (*Plus and Minus* ou *Geometric*), tipo de cota de padrão (linear, polar); definir os datums primário, secundário e terciário; selecionar as features; revisar. O esquema geométrico impõe: planeza no primário, perpendicularidade no secundário em relação a A, posição no terciário em relação a A e B [V].
- **Show Tolerance Status (CAD avançado A):** face verde = totalmente restringida, amarela = sub, vermelha = sobre, cor original = sem dado [V].
- **CAD avançado C:** *Tolerance Advisor* com 39 mensagens e *Face Status Coloring* desde 2019 [V].
- **CAD avançado F:** *Advisor Tree* com graus de liberdade pendentes; verde/cinza/azul (coberta pela nota de perfil geral) [V]. *Create Selected* gera as cotas de furos, eixos, rasgos, placas e furos oblongos [V].
- **CAD avançado G:** *MBD logical rules* na biblioteca de reutilização, com pastas Dimensions, Datums e Feature Control Frames [V]. Duplo clique na regra e responder aos prompts aplica as anotações.
- **CAD avançado B:** *Generating Check Report* e o Advisor que bloqueia tolerâncias inválidas [V].

**Fenkra hoje.** Nada além de balões automáticos em montagem (`auto_balloons`, `pmi_ui.rs:140`). B38 (P3) cobre o consultor, mas só em uma linha.

**Regras que evitam erro:**
- Dois níveis: (1) assistente de DRF + esquema por peça, tudo editável e marcado "gerado", regenerável sem apagar edições manuais; (2) regras de PMI reutilizáveis por tipo de feature ("furo passante → Ø com H12 + nota de rosca").
- Verificação em três camadas: sintática (sempre ligada), de esquema (sob demanda; graus de liberdade, faces sem requisito) e de norma (símbolos removidos).
- "Completo" é meta opcional: oferecer modo "tudo explícito" e "mínimo para fabricar" (princípio do CAD avançado A).
- Colorir faces (verde/amarelo/vermelho/cinza) com clique que realça a anotação.

**Saída AP242.** Não aplicável (é fluxo de autoria), mas o resultado **gerado** sai como PMI comum. As propriedades de validação (RP §10) são geradas na exportação [V].

---

## 3. MVP recomendado, em ordem

O critério é "o que dá resultado errado primeiro, depois o que destrava o resto". A ordem segue as dependências do backlog (B82, B83 antes de tudo).

**Fase 0: pré-requisitos já no backlog (não repetir)**
1. **B82** (alvos topológicos e medida por alvo) e **B83** (estado e diagnóstico): sem eles qualquer tipo novo nasce quebrado, porque `snap` de 2 mm não resolve face nem centro de furo (`pmi_ui.rs:243`).
2. **B81** (âncoras estáveis do desenho): a cota 2D e a 3D compartilham o tipo `Anchor`.

**Fase 1: dados normativos e tolerância tipada (barato, destrava o resto)**
3. **Crate de tolerâncias normativas** (ISO 286-1/-2, ISO 2768-1/-2, graus IT): tabelas versionadas, funções puras, teste contra valores publicados. É a base de B108 e do AP242 `limits_and_fits`.
4. **B108** reduzido a tolerância tipada, Datum/DRF e advisor por elemento (o FCF completo passa a item próprio, abaixo).

**Fase 2: catálogo de cotas**
5. **Cotas por par de alvos** (G1–G3): face–face, eixo–eixo, ao longo de eixo, ordenada/base/cadeia/simétrica, arco/corda, raio/Ø de esfera, ângulo entre faces/arestas com 4 modos. Mínimo viável: face–face, eixo–eixo, arco, ângulo entre faces e as 3 conjuntos.
6. **Cotas de feature lidas do modelo** (G4–G5): furo, rebaixo, escareado, rosca, padrão `n×`, chanfro, filete, espessura de chapa. Aproveita `Hole`/`Thread`/chapa. Compartilhar o gerador de texto com B107 (desenho), para que 3D e 2D mostrem a mesma chamada.

**Fase 3: GD&T completo e alvos**
7. **FCF completo** (G8): modelo tipado de zona, modificadores enum, quadro composto, valor máximo, por unidade, perfil dinâmico, CF, símbolos vetoriais, matriz de validade ISO/ASME.
8. **Datum targets e áreas restritas** (G9), com alvos móveis.
9. **Acabamento e solda estruturados** (G10): rugosidade ISO 21920/1302/ASME Y14.36 ligada à face e solda sistema B/AWS A2.4.

**Fase 4: saída e automação**
10. **AP242 semântico** pelo XCAF (depende de B87): cotas, tolerâncias, datums, datum targets via `XCAFDoc_DimTolTool`, mais PMI gráfico para o que não tem semântica (rugosidade, solda, notas). Fechar com releitura do próprio arquivo no OCCT e checagem no NIST STEP File Analyzer.
11. **Esquema de cotagem automático** (G11) e evolução de **B38** para coloração de estado das faces.

**Critérios de pronto por fase:** cada tipo nasce com peça no corpus B53, teste "muda a cota e volta", salvar/abrir idêntico e, a partir da fase 4, exportar e reler.

---

## 4. Fontes

**OCCT / AP242** [V]
- [XCAFDimTolObjects_DimensionType.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DimensionType.hxx) (32 valores)
- [XCAFDimTolObjects_DimensionModif.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DimensionModif.hxx) (24)
- [XCAFDimTolObjects_DimensionQualifier.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DimensionQualifier.hxx)
- [XCAFDimTolObjects_GeomToleranceType.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_GeomToleranceType.hxx) (16)
- [XCAFDimTolObjects_GeomToleranceModif.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_GeomToleranceModif.hxx) (17)
- [XCAFDimTolObjects_GeomToleranceTypeValue.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_GeomToleranceTypeValue.hxx)
- [XCAFDimTolObjects_DatumSingleModif.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DatumSingleModif.hxx) (22)
- [XCAFDimTolObjects_DatumModifWithValue.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DatumModifWithValue.hxx)
- [XCAFDimTolObjects_DatumTargetType.hxx](https://raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/DataExchange/TKXCAF/XCAFDimTolObjects/XCAFDimTolObjects_DatumTargetType.hxx)
- [CAx-IF Recommended Practices for the Representation and Presentation of PMI (AP242) v4.1, 20/06/2024](https://www.mbx-if.org/home/wp-content/uploads/2024/06/rec_pracs_pmi_v41.pdf): seções 5.1 a 5.4, 6.3 a 6.9, 7, 8, 9.4.
- [NIST STEP File Analyzer and Viewer](https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer)

**CAD avançado B** [V parcial]
- Dimensions (CAD avançado B Drafting, lista de comandos)
- Creating Curvilinear Length Dimensions
- Creating Chamfer Dimensions
- [IBM APAR HD51528 (chamfer dimension em superfícies curvas)](https://www.ibm.com/support/pages/apar/HD51528)
- [IBM Support 5342649 (Linear Size via Semantic Tolerancing Advisor)](https://www.ibm.com/support/pages/node/5342649)
- [3DCS: Using the Tolerance Advisor / Useful FT&A settings](https://community.3dcs.com/help_manual/v5ftacreation.htm)
- Ascent: CAD avançado B-6R2023 Functional Tolerancing and Annotation

**CAD avançado A MBD / DimXpert** [V parcial]
- [Engineers Rule: chanfro, cone e cota de referência no MBD](https://www.engineersrule.com/?p=1241)
- DimXpert Size Dimension: features suportadas (resultado de busca; página não renderiza)
- DimXpert Location Dimension
- CAD avançado A MBD 2023 (GoEngineer)
- CAD avançado A 2024 What's New: MBD
- CAD avançado A 2025: símbolos ISO 21920
- Central Innovation: CAD avançado A MBD

**CAD avançado C / Fusion**
- Sobre anotação 3D e MBD
- CAD avançado C 2024: anotação 3D e MBD
- CAD avançado C 2027: melhorias de MBD
- fornecedor de CAD avançado: CAD avançado C 3D Annotations (blog): só o resumo da busca; página deu 403.
- Fusion: PMI (blog fornecedor de CAD avançado): só o resumo da busca; página deu 403.
- Fusion API: PMIAnnotation

**CAD avançado F**
- Limitações do GD&T Advisor (R12)
- GD&T Advisor Enhancements (R9)

**CAD avançado G**
- CAD avançado G Tips and Tricks: PMI
- NXOpen OrdinateDimension e ChainDimension (só o resumo da busca)
- [TU Darmstadt: PMI no CAD avançado G, anotações](https://www.iim.maschinenbau.tu-darmstadt.de/CAD-Tutorial/9_pmi/1_pmi_on_stangen_querverbinder/5_annotations/index.en.html)

**FreeCAD**
- [TechDraw Workbench](https://wiki.freecad.org/TechDraw_Workbench/id) e [TechDraw HorizontalExtentDimension](https://wiki.freecad.org/TechDraw_HorizontalExtentDimension): só resumo da busca; a página direta foi bloqueada. O workbench Assembly e o TechDraw não têm PMI 3D [C].

**Normas**
- [ASME Y14.5 2009 vs 2018 (gdandtbasics)](https://www.gdandtbasics.com/asme-2009-vs-2018/)
- [ISO 21920-1:2021 (AFNOR)](https://www.boutique.afnor.org/en-gb/standard/iso-2192012021/geometrical-product-specifications-gps-surface-texture-profile-part-1-indic/xs136451/318317)
- [ISO 14405-1:2025 (AFNOR)](https://www.boutique.afnor.org/en-gb/standard/iso-1440512025/geometrical-product-specifications-gps-dimensional-tolerancing-part-1-linea/xs145506/448924)

**Código do Fenkra lido:** `crates/fenkra-core/src/pmi.rs` (linhas 1–880), `crates/fenkra-app/src/pmi_ui.rs` (1–260 e 1055–1100), `crates/fenkra-desenho/src/lib.rs` (230–420), `crates/fenkra-core/src/doc.rs` (2540–2584), `docs/pesquisa/pmi.md`, `docs/benchmark/pmi-3d.md` e `docs/backlog/backlog.md` (itens B23, B38, B81, B82, B83, B87, B107, B108, B117, B126, B144, B149).

---

## 5. Limites, riscos e handoff

- **Resultado:** catálogo de cerca de 90 tipos, matriz por software, fichas por grupo e MVP em 11 passos, com 8 itens novos e 9 ajustes de backlog no JSON abaixo.
- **Arquivos alterados:** nenhum no projeto. O PDF do CAx-IF foi baixado pela ferramenta de leitura e extraído para texto no scratchpad.
- **Checagens feitas:** leitura dos enums do OCCT direto do GitHub; leitura do texto do RP v4.1 (seções 5.1–5.4, 6.3–6.9); conferência das linhas citadas do Fenkra.
- **Não conseguido:**
  - leitura direta da ajuda do CAD avançado A, do guia FT&A do CAD avançado B, dos docs do CAD avançado G e do FreeCAD;
  - lista oficial de cotas do Fusion;
  - significado dos códigos ISO 14405-1 (LP, GG, GX…).
  
  Essas células da matriz estão como ☑ ou ❔, não ✅.
- **Riscos:**
  - O CAx-IF não cobre rugosidade, solda, chanfro nem rosca como semântica. Quem promete "AP242 completo" não terá nada para ler essas anotações em outro CAD além do gráfico.
  - O OCCT não tem enum de perfil dinâmico, de CZ para datum nem de alguns modificadores ISO 1101:2017. O que o AP242 permite pode ser maior do que o `XCAFDoc_DimTolTool` expõe; validar com um STEP de teste antes de prometer.
  - A afirmação sobre Ⓢ (item 9 da ficha G8) é [C] e precisa ser conferida na norma.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Catálogo de ~90 tipos de cota e anotação 3D comparado entre CAD avançado B, CAD avançado A, CAD avançado C, CAD avançado F, CAD avançado G, CAD avançado D e FreeCAD, com o gap do Fenkra (cotas só ponto a ponto, tolerância em texto, sem datum target nem AP242 semântico) e 8 itens novos; as células do CAD avançado B, CAD avançado A, CAD avançado G, CAD avançado D e FreeCAD vêm de fontes secundárias, porque as ajudas oficiais não abriram.
