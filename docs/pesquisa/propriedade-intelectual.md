# Auditoria de propriedade intelectual

> Gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas.

**Projeto:** Fenkra CAD, versão `0.1.0-alpha.28`, branch `develop`, repositório `github.com/JvDNasci/fenkra`
**Data:** 01/11/2026 · **Método:** leitura do repositório, do zip de release e do vcpkg local, mais pesquisa em fontes públicas na web.

> **Aviso.** Isto **não é parecer jurídico nem aconselhamento jurídico.** É um levantamento de riscos para o autor levar a um advogado de propriedade intelectual, de preferência com prática em software e marcas no Brasil, nos EUA e na UE. Onde a pesquisa não conseguiu confirmar um fato, o texto diz isso. Leis e decisões foram lidas em resumo, por páginas públicas.

---

## 1) Resumo em linguagem simples

**O autor está ferindo direitos de alguém? Pelo que se vê hoje, não de forma evidente.** O trabalho de estudar CAD avançado B, CAD avançado A e CAD avançado C, entender os fluxos e reescrever tudo com palavras próprias é, em regra, permitido. Ideias, métodos, funcionalidade e a lógica de um comando não são protegidos por direito autoral. Isso vale no Brasil (Lei 9.610/98, art. 8º, I; Lei 9.609/98, art. 6º, III) e na UE, e é a linha geral nos EUA (*Lotus v. Borland*; *Google v. Oracle*).

**Os riscos reais não estão em copiar funcionalidade. Estão em coisas de "empacotamento" que ainda faltam ou precisam de decisão:**

1. **Não existe `LICENSE`, `NOTICE` nem `THIRD-PARTY` no repositório nem no zip.**
   - O zip `alpha.28` tem 34 arquivos: o executável e 33 DLLs, sem nenhum documento.
   - O repositório está público no GitHub, sem licença exibida.
   - Sem arquivo de licença, o código fica, na prática, com "todos os direitos reservados", ainda que o `Cargo.toml` diga LGPL.
   - Ao distribuir o OCCT, o FreeType, o libpng e outros, o autor **tem obrigações de aviso e de oferta de código-fonte que hoje não são cumpridas.** Este é o ponto mais concreto e mais fácil de resolver.
2. **O ícone do app (a fênix) nasceu de uma imagem gerada por IA** com o selo "Made with AI". A ferramenta `vetorizar` remove esse selo e traça os contornos.
   - A origem declarada no script `gerar_fenix.py` ("desenho original, gerado por código") não bate com isso.
   - Falta saber qual ferramenta gerou a imagem e quais são os termos dela.
   - Obra gerada só por IA tende a não ter proteção autoral. Isso enfraquece o logo como ativo.
3. **O nome "Fenkra" não foi checado em bases de marcas.** A pesquisa web só achou o próprio repositório, mas INPI, USPTO e EUIPO não são indexados por buscadores. A conclusão é "inconclusivo", não "livre".
4. **A interface usa nomes de marcas de terceiros**, como "CAD avançado A", "CAD avançado B", "CAD avançado C", "CAD avançado D", "CAD avançado E" e "Blender". Aparecem em presets de atalhos e de navegação.
   - É uso **referencial** (sem logos), o que costuma ser defensável.
   - Mas contradiz a regra escrita no próprio README ("não usar nomes que são marcas registradas") e exige um aviso de marcas.
5. **Material "bruto" gerado por IA está commitado:**
   - `docs/contextos/*` (por exemplo `smart-fastener.md`, que descreve passo a passo o biblioteca de normas e o fixadores inteligentes do CAD avançado A);
   - cerca de 217 mil palavras de `docs/pesquisa/*`, gravadas "como cada worker entregou".
   - O risco de cópia literal é baixo nas amostras lidas, mas ninguém o verificou de ponta a ponta, e o registro de procedência tem só 3 linhas.
6. **Dados de normas e catálogos.**
   - Medidas são fatos, e a política (ADR 0008) está correta.
   - Mas uma das fontes (fasten.it) declara, no rodapé, que reproduz tabelas "por permissão do DIN". O DIN afirma que copiar tabelas literalmente infringe direito autoral.
   - O volume atual é pequeno (7 arquivos, M3 a M20), então o risco é baixo. Cresce se a biblioteca for ampliada copiando tabelas inteiras.
7. **Dados do FreeCAD Fasteners (GPL-2.0)** estão corretamente tratados como "só conferência" (nível X). Não encontrei dado copiado deles.

**Onde NÃO há risco relevante (com o que foi visto):**
- As dependências Rust. Nas 266 crates do build Windows não há GPL-only nem AGPL. Só aparece uma dupla "Apache-2.0 OR GPL-2.0-only", em que se escolhe a Apache.
- O OCCT. Ele é ligado dinamicamente (DLLs), o que é o caminho recomendado pela LGPL, e tem uma exceção própria bem favorável.
- Os 356 ícones de comando. São gerados por código, sem importar SVG externo.
- Os screenshots de `docs/contextos/review`. Os 5 que abri são do próprio Fenkra.

**Patentes:** não achei patente de CAD que o Fenkra comprovadamente infrinja, mas **não consegui verificar** as patentes da fornecedor de CAD avançado (*synchronous technology*). Há um pedido pendente da fornecedor de CAD avançado sobre sugestão de mates ao arrastar e pausar. Isso só se resolve com uma busca de liberdade de operação (FTO) feita por profissional, antes de vender, principalmente nos EUA.

---

## 2) Matriz de riscos

| # | Item | Risco | Por quê | Ação recomendada |
|---|---|---|---|---|
| 1 | Ausência de `LICENSE`/`NOTICE`/`THIRD-PARTY` (repo e zip) | **Alto** | Sem licença o código fica "todos os direitos reservados". O zip distribui DLLs LGPL, FTL, zlib e outras sem avisos nem texto de licença. | Criar os arquivos e incluí-los no zip. Ajustar `release.ps1` para **falhar** se faltarem. |
| 2 | Oferta de código-fonte do OCCT e relink (LGPL-2.1 §6) | **Alto** (hoje não cumprido) | Distribuir binários do OCCT 8.0.1 exige oferta ou acesso ao código e aviso proeminente. Hoje não há nada. | Texto de oferta no `NOTICE`, link para o OCCT 8.0.1 e para a porta vcpkg (por hash), e as instruções de build. |
| 3 | Logo da fênix (imagem de IA vetorizada, selo removido) | **Médio** | Proteção autoral incerta, termos da ferramenta de IA desconhecidos, possível semelhança com obra existente, procedência contraditória. | Identificar a ferramenta e guardar os termos e o prompt. Considerar refazer o logo com um designer. Corrigir os comentários dos scripts. |
| 4 | Nome "Fenkra" sem busca de marca | **Médio** (inconclusivo) | Não foi possível consultar INPI, USPTO e EUIPO. Há risco de conflito em classes 9 e 42. | Busca oficial, incluindo variantes fonéticas. Depositar no INPI (primeiro a depositar) antes de comercializar. |
| 5 | Nomes de marcas na UI (presets "CAD avançado A", "CAD avançado B", "CAD avançado C" etc.) | **Baixo–Médio** | Uso referencial, sem logo. O DS exige autorização escrita para "usar" as marcas em seu site. A LPI art. 132 IV protege citação "sem conotação comercial". Em produto vendido, a defesa é mais fraca. | Manter só como texto referencial ("atalhos compatíveis com…"). Adicionar aviso de marcas na tela "Sobre". Alinhar a regra do README. Pedir opinião do advogado. |
| 6 | Atalhos, estilos de navegação e nomes de comandos copiados em função | **Baixo** | Funcionalidade, métodos e leiaute de comandos não são protegidos (art. 8º I da Lei 9.610; *Lotus*; SAS v. WPL). | Manter a política de reescrever. Evitar copiar rótulos compostos exclusivos de uma marca. |
| 7 | Tópicos de docs com nomes de produtos (assistente de furos, fixadores inteligentes etc., só em `docs/`) | **Baixo** | Uso comparativo e descritivo em pesquisa. Nenhum desses nomes aparece em `crates/`. | Aviso de marcas em `docs/README.md`. Não usar esses nomes como nome de comando. |
| 8 | `docs/contextos/*` (texto gerado por IA, "bruto") | **Médio–Baixo** | Pode reproduzir, quase literalmente, ajudas e tutoriais de terceiros. O `smart-fastener.md` é um tutorial detalhado do CAD avançado A. | Revisar ou reescrever antes de publicar. Se não for essencial, tirar do repositório público. |
| 9 | `docs/pesquisa/*` (217 mil palavras geradas por IA) | **Baixo–Médio** | As amostras estão parafraseadas, com fontes e etiquetas [V]/[C]. Mas a verificação foi só por amostragem, e o registro de procedência tem 3 linhas. | Varredura de similaridade das frases mais longas. Completar `procedencia.md`. |
| 10 | EULAs e termos (CAD avançado A, fornecedor de CAD avançado, CAD avançado B) | **Médio** (depende de fatos) | Quem aceitou um EULA pode ter restrições contratuais (engenharia reversa, benchmark). Ler ajuda pública não aceita EULA. Não consegui ler os EULAs completos. | Perguntar ao autor se o conhecimento veio de instalações licenciadas. Levar ao advogado. |
| 11 | Dados de normas (ISO/DIN/ABNT) no `biblioteca/` | **Baixo** agora, **Médio** se escalar | Fatos dimensionais são livres em regra. O DIN diz que copiar tabelas literalmente infringe. Fontes agregadoras reproduzem "com permissão do DIN". | Manter política A/B/C. Não copiar layout, ordem nem notas. Registrar a fonte por valor. |
| 12 | Rótulo "CC BY 4.0 (dados)" nos JSON | **Baixo** | Licenciar fatos de terceiros como se fossem nossos pode ser inócuo ou enganoso. | Rotular como "compilação do Fenkra, CC BY 4.0; valores dimensionais são fatos de norma". |
| 13 | Catálogos de fabricantes (Gerdau, Aço Brasil, ArcelorMittal, AISC) | **Médio** (não verificado) | Não encontrei os termos de uso da Gerdau. Os do AISC retornaram 403 (`fontes-de-dados.md:25`). Hoje não há dados desses catálogos no repo. | Ler os termos antes de importar. Extrair só medidas, por família, com a página citada. |
| 14 | FreeCAD Fasteners (GPL-2.0) | **Baixo** | Política X correta. Não achei dado copiado. GPL-2.0 não combina com a LGPL-2.1 do projeto. | Manter a vedação. Adicionar uma checagem no CI contra arquivos com origem GPL. |
| 15 | Combinação de crates Apache-2.0 com projeto LGPL-2.1-or-later | **Baixo–Médio** | Apache-2.0 não é compatível com GPLv2, mas é com GPLv3. O "or-later" permite escolher a v3, o que ajuda. | Perguntar ao advogado e documentar no `NOTICE` a escolha de licença efetiva do binário. |
| 16 | Fontes tipográficas embutidas (Ubuntu Light, Noto Emoji, Hack) | **Baixo** | UFL-1.0 e OFL-1.1 exigem aviso e texto de licença por cópia da fonte. | Incluir os textos no `THIRD-PARTY-LICENSES`. |
| 17 | Fontes do Windows lidas em tempo de execução (`C:\Windows\Fonts`) | **Baixo** | Não são redistribuídas. Uso local pelo usuário, que as tem licenciadas. | Nada a fazer. Não empacotar essas fontes. |
| 18 | Patentes (fornecedor de CAD avançado, fornecedor de CAD avançado, outras) | **Desconhecido** | Pedido pendente da fornecedor de CAD avançado (mate sugerido ao arrastar e pausar). Patentes da fornecedor de CAD avançado não verificadas. | FTO por profissional antes de vender nos EUA e na UE. |
| 19 | Formatos de arquivo (STEP, IGES, DXF, DWG, JT, nativos) | **Baixo** | Exporta STEP/STL/DXF/PDF/SVG. DWG e nativos estão fora do escopo. JT tem licença de patente da fornecedor de CAD avançado restrita a implementações conformes. | Não usar "DWG" como marca nem "compatível com DWG" em marketing. Manter nativos como não objetivo. |
| 20 | Contribuições externas (sem `CONTRIBUTING`, sem DCO/CLA) | **Médio** (futuro) | Hoje há 1 autor (120 commits). Quando entrarem terceiros, a licença fica difícil de mudar. | Adotar DCO ou CLA antes do primeiro PR externo. |
| 21 | Nomes antigos (SolidCadCraft) em `dist/` | **Baixo** | A pasta `dist/` é ignorada pelo git. Zips antigos podem ter circulado. | Apagar ou não distribuir os zips antigos. Registrar a renomeação. |

---

## 2.1 Evidências e análise

### (a) `LICENSE` / `NOTICE` / `THIRD-PARTY` na raiz e no zip

**Raiz.** `ls LICENSE* NOTICE* THIRD* COPYING*` não encontra nada. `git ls-files` não lista nenhum arquivo `LICENSE`, `NOTICE`, `THIRD_PARTY`, `CHANGELOG`, `packaging/*` nem `CONTRIB*`. Também não há pasta `.github`, então não há CI.

**O que existe sobre licença:**
- `Cargo.toml:16` → `license = "LGPL-2.1-or-later"`.
- `docs/arquitetura/adr/0003-licenca.md:3` → status **"proposta (decisão do mantenedor)"**. A linha 24 recomenda consultar um advogado antes da primeira versão pública.
- `docs/arquitetura/reuso-opensource.md:17` → "registrar a origem em `THIRD_PARTY.md` (a criar)".
- `docs/backlog/backlog.md:38` → item **B67** já prevê "LICENSE, THIRD_PARTY.md e cargo-deny sem GPL no binário", como `a fazer`.

**Zip de release.** `scripts/release.ps1:64-68` copia "documentos opcionais": `CHANGELOG.md`, `packaging\LEIA-ME.txt` e `packaging\TERCEIROS.txt`, só `if (Test-Path ...)`.
- **Nenhum desses arquivos existe.** A pasta `packaging/` não existe.
- O script, portanto, **não falha**: ele gera um pacote sem documento algum.
- O zip `dist/Fenkra-0.1.0-alpha.28-windows-x64.zip` tem 34 entradas: `fenkra.exe` e 33 DLLs, 77,4 MB descompactado. Nem README nem licença.

**GitHub.** A página `github.com/JvDNasci/fenkra` mostra "Public", sem licença na barra lateral, sem releases e sem `LICENSE`/`NOTICE` na raiz.
- Como regra geral, código público sem licença não dá a terceiros direito de uso, cópia ou modificação além do que os termos do GitHub permitem (ver e fazer fork na plataforma). Essa regra eu **não pesquisei** aqui; confirme com o advogado.
- Vale para o autor: hoje ele tem 120 commits e é o único autor (`git log`). Isso torna **fácil e sem fricção** escolher e aplicar a licença agora.

**Cabeçalhos de arquivo.** Nenhum `.rs` tem linha de copyright ou SPDX (busca por `Copyright|SPDX` em `crates/` não retorna nada). A LGPL recomenda aviso por arquivo. Um `SPDX-License-Identifier: LGPL-2.1-or-later` no topo de cada arquivo é a forma mais barata.

### (b) Licenças das dependências

#### Crates Rust

Rodei `cargo metadata --locked --offline --filter-platform x86_64-pc-windows-msvc` (somente leitura). Resultado: **266 pacotes** no conjunto Windows. O `Cargo.lock` tem 486 entradas porque inclui crates de outras plataformas (Linux, macOS, wasm). Essas **não** foram resolvidas offline. Uma verificação completa precisa de `cargo-deny` ou `cargo-about` com rede.

Distribuição das licenças declaradas no conjunto Windows:

| Licença (SPDX declarado) | Pacotes |
|---|---|
| MIT OR Apache-2.0 | 136 |
| MIT | 28 |
| Apache-2.0 OR MIT | 26 |
| Unicode-3.0 (ICU: `icu_*`, `zerovec`, `yoke` etc.) | 18 |
| Apache-2.0 | 10 |
| MIT/Apache-2.0 | 7 |
| **LGPL-2.1-or-later** | **7 (os crates do próprio Fenkra)** |
| MIT OR Apache-2.0 OR Zlib | 5 |
| Unlicense OR MIT | 4 |
| BSL-1.0 (`clipboard-win`, `error-code`) | 2 |
| Zlib (`foldhash`, `zlib-rs`) | 2 |
| BSD-3-Clause (`tiny-skia`, `tiny-skia-path`) | 2 |
| BSD-3-Clause OR Apache-2.0 | 2 |
| Zlib OR Apache-2.0 OR MIT | 2 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 2 |
| Demais combinações de MIT, Apache, CC0, ISC, 0BSD, BSD-2 | 14 |
| **Apache-2.0 OR GPL-2.0-only** (`self_cell 1.3.0`) | 1 |
| **(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0** (`epaint_default_fonts 0.37.0`) | 1 |

**Leitura:**
- **Não há GPL-only, AGPL nem LGPL de terceiros** no conjunto Windows. Das 266, só as 7 LGPL são do próprio Fenkra.
- `self_cell` oferece "Apache-2.0 **OR** GPL-2.0-only". Escolhe-se Apache-2.0 e registra-se isso no `THIRD-PARTY`.
- `epaint_default_fonts` empacota fontes com licenças próprias. O crate traz em `fonts/` os arquivos `OFL.txt`, `UFL.txt`, `Hack-Regular.txt`, `egui-icons.txt` e `emoji-icon-font-mit-license.txt`. Esses textos precisam acompanhar o binário.
- MIT, BSD e Apache exigem **reproduzir o aviso de copyright e o texto da licença** ao distribuir binários. Apache-2.0 exige também o conteúdo dos `NOTICE` upstream, quando existirem (§4).
- Dependências diretas (`Cargo.toml` e `crates/*/Cargo.toml`): `serde`, `serde_json`, `thiserror`, `glam`, `bytemuck`, `cxx`/`cxx-build`, `eframe` (egui 0.37, com wgpu), `egui_extras`, `image`, `rfd`, `blake3`, `winresource` (build). `tools/vetorizar` é um workspace separado (`image`, `imageproc`) e não vai no pacote.

#### OCCT e as 33 DLLs do release

O vcpkg local (`C:\dev\vcpkg\installed\x64-windows-release\share`) traz `copyright` e `vcpkg.spdx.json` por pacote. O OCCT é a versão **8.0.1**.

| DLLs no zip | Biblioteca (versão no vcpkg) | Licença |
|---|---|---|
| `TKernel`, `TKMath`, `TKG2d`, `TKG3d`, `TKGeomBase`, `TKBRep`, `TKGeomAlgo`, `TKTopAlgo`, `TKPrim`, `TKBO`, `TKBool`, `TKFillet`, `TKOffset`, `TKMesh`, `TKShHealing`, `TKHLR`, `TKService`, `TKV3d`, `TKCDF`, `TKLCAF`, `TKCAF`, `TKVCAF`, `TKXCAF`, `TKXSBase`, `TKDE`, `TKDESTEP`, `TKDESTL` (**27 DLLs**) | Open CASCADE Technology 8.0.1 | **LGPL-2.1 + exceção OCCT 1.0** (`share/opencascade/copyright`: `LICENSE_LGPL_21.txt` e `OCCT_LGPL_EXCEPTION.txt`, linhas 507-520) |
| `freetype.dll` | FreeType 2.14.3 | **FTL ou GPLv2, à escolha** (`share/freetype/copyright`, linhas 15-28). Escolher FTL. A FTL exige a frase de crédito "Portions of this software are copyright © <ano> The FreeType Project (https://freetype.org). All rights reserved." (linhas 104-105) |
| `libpng16.dll` | libpng 1.6.59 | PNG Reference Library License v2 (aviso e texto) |
| `z.dll` | zlib 1.3.2 | Licença zlib (permissiva; a licença pede "agradecimento na documentação", se usado em produto) |
| `bz2.dll` | bzip2 1.0.8 | Licença BSD-like do bzip2 (aviso, texto e não deturpar a origem) |
| `brotlicommon.dll`, `brotlidec.dll` | Brotli 1.2.0 | MIT |

O zip também contém `fenkra.exe` (32 MB), que embute as crates Rust e a fachada C++ própria (`crates/fenkra-kernel/cpp/*`).

**Como o OCCT é ligado.** `crates/fenkra-kernel/build.rs` liga cada `TK*.lib` como `dylib` (`cargo:rustc-link-lib=dylib=...`) e copia as DLLs para a pasta do executável. Isso é **ligação dinâmica**. É o cenário que a LGPL-2.1 §6(b) chama de "mecanismo adequado de biblioteca compartilhada": o usuário pode trocar as DLLs do OCCT por versão modificada e compatível.

**A exceção do OCCT** (`OCCT_LGPL_EXCEPTION.txt`) permite distribuir o código objeto de um "trabalho que usa a Biblioteca" que incorpore material de headers do OCCT sob os termos que o distribuidor escolher, **desde que** haja "aviso proeminente na documentação de apoio" de que o produto usa ou se baseia no OCCT. O aviso na tela de abertura (`app.rs:995`: "Iniciando o kernel OpenCASCADE…") é só uma mensagem de progresso. **O zip não tem documentação de apoio.** A condição da exceção não está atendida hoje.

**Obrigações da LGPL-2.1 §6 ao distribuir o zip** (texto lido em gnu.org):
- Aviso proeminente de que a biblioteca é usada e coberta pela licença.
- Cópia da licença.
- Aviso de copyright da biblioteca onde o programa exibir avisos de copyright.
- Uma entre as alíneas a)–e): código-fonte completo, shared library, **oferta escrita válida por 3 anos**, acesso equivalente ao mesmo local, ou verificação de que o usuário já recebeu o material.
- A §6 também exige que os termos do trabalho que usa a biblioteca **permitam modificar o trabalho para uso próprio do cliente e engenharia reversa para depurar essas modificações.** Portanto o Fenkra **não deve** ter um EULA que proíba engenharia reversa.

**Escolha de licença efetiva.** A LGPL-2.1 §3 e o "or-later" permitem aplicar a GPL ou versões posteriores. A combinação com crates Apache-2.0 (winit, glutin, accesskit_winit etc.) tem um ponto sutil. O FSF considera a Apache-2.0 compatível com a GPLv3 e não com a GPLv2; a LGPL-2.1-or-later permite o caminho da v3. Eu não consegui confirmar a análise específica para LGPL-2.1-or-later; está na lista do advogado.

**Fachada C++ própria** (`kernel.cpp` e `.inc`): é código do próprio autor, linkado ao OCCT por headers e DLLs. Cai na exceção citada.

**Runtime do MSVC.** O executável é compilado com Visual Studio. Eu **não verifiquei** se depende da VC++ Redistributable instalada no sistema nem sob que termos ela é distribuída. Fica como pergunta de checklist.

### (c) Nomes de marcas na interface e na documentação

**Código** (`crates/`):

| Local | O que aparece |
|---|---|
| `keymap.rs:2`, `172-207` | Presets de atalhos nomeados `CAD avançado A`, `CAD avançado B`, `FreeCad`, `Fusion`, `Blender`, `CAD avançado E`, `CAD avançado C`; rótulos `tr("CAD avançado A")`, `tr("CAD avançado B")`, `tr("CAD avançado D")`, `tr("CAD avançado C")` (linhas 187-193) |
| `settings.rs:217-223`, `230-236` | "Estilo CAD avançado A", "Estilo CAD avançado B", "Estilo FreeCAD (CAD)", "Estilo CAD avançado C" e descrições do mouse |
| `i18n.rs:232-239` | Os mesmos rótulos (e "Estilo CAD avançado I") com tradução inglesa |
| `app.rs:5413-5439` | Lógica de navegação por `N::CAD avançado A`, `N::CAD avançado C`, `N::Blender`, `N::CAD avançado E` |
| `i18n.rs:1932` e `toolbar.rs:498` | Dica de seleção: "como no CAD avançado J" |
| `app.rs:7186`, `doc.rs:400` | Comentários "como os tipos de linha do CAD avançado J" |
| `engrenagens.rs:1` | Comentário "estilo Design Accelerator" |
| `i18n.rs:154`, `i18n.rs:1581` | "Cotagem inteligente (D)" / "Smart dimension (D)"; "Repetição inteligente de componente" / "Smart component pattern" |
| `asm_ui.rs:568`, `icons.rs:45`, `display.rs:1` | Identificadores internos `AsmComposer`, ícone `composer.svg`, comentário "estilo composer" |
| `asm_ui.rs`, `lib.rs:245` | Identificador `Mate` (termo genérico de montagem; **não** é "SmartMates") |

Não há, em `crates/`, nenhuma ocorrência de **"assistente de furos"**, **"fixadores inteligentes"**, **"objeto de trabalho"**, **"Instant3D"** nem **"biblioteca de normas"**. A busca nessas strings em `crates/` não retornou nada. O rótulo visível do recurso de exibição é "Exibição" (`toolbar.rs:412`), não "Composer".

**Documentação.** Contagem de arquivos de `docs/`, `README.md`, `scripts/`, `assets/` e `biblioteca/` que citam cada termo:

| Termo | Arquivos |
|---|---|
| CAD avançado B | 52 |
| CAD avançado C | 48 |
| CAD avançado A | 29 |
| CAD avançado E | 22 |
| CAD avançado F | 19 |
| assistente de furos | 19 |
| biblioteca de normas | 19 |
| fornecedor de CAD avançado | 16 |
| fixadores inteligentes | 14 |
| CAD avançado H | 12 |
| Design Accelerator | 12 |
| Define In Work (Object) | 10 |
| fornecedor de CAD avançado | 5 |
| plataforma PLM avançada | 5 |
| CAD avançado D | 3 |
| kernel geométrico proprietário | 3 |
| Instant3D | 2 |

Esses usos estão em documentos de **pesquisa e benchmark comparativo**, que são o contexto típico de uso referencial.

**O que dizem as regras públicas:**
- A **LPI (Lei 9.279/96)**, art. 132, IV, diz que o titular não pode impedir "a citação da marca em discurso, obra científica ou literária ou qualquer outra publicação", desde que **sem conotação comercial** e sem prejuízo ao caráter distintivo. O art. 132, II, trata de fabricantes de acessórios que indicam a destinação do produto, "observadas as práticas leais de concorrência". Os arts. 125 e 126 dão proteção especial a marcas de alto renome e notoriamente conhecidas. Os arts. 189 e 190 tipificam a reprodução e a imitação de marca registrada.
- O texto dos termos do site do **CAD avançado A** diz que o uso de marcas da fornecedor de CAD avançado "está sujeito à sua aprovação expressa por escrito" e remete às diretrizes de marca. O manual de parceiros diz que o **logotipo** só é autorizado a funcionários e revendedores contratados, e que o texto simples deve escrever CAD avançado A em maiúsculas.
- A **fornecedor de CAD avançado** diz que o uso referencial do nome é geralmente permitido, desde que sem logotipo nem ícone de produto, sem implicar endosso, e sem símbolos ™/® em marcas de terceiros. Trata **DWG** como marca própria. Não pude abrir a página das diretrizes completa (HTTP 403); a informação vem do trecho de busca.
- Não encontrei registros de marca para "assistente de furos", "fixadores inteligentes", "biblioteca de normas" ou "SmartMates" nos USPTO; a busca é inconclusiva (as bases não são indexadas). Trate esses nomes como possivelmente protegidos, e não os use.
- "Hi-Lok" aparece como marca da Hi-Shear Corporation na página da Howmet. "CherryMAX" consta como marca registrada de SPS Technologies. "Hi-Lite": sem registro encontrado. Esses nomes só aparecem em `docs/pesquisa/prendedores-aeronauticos.md` (por exemplo as linhas 3, 55, 60, 178), não em `crates/` nem em `biblioteca/`.

**Avaliação.** Os presets são **uso nominativo/referencial** de marcas conhecidas para indicar compatibilidade de atalhos, sem logos. Isso é, em geral, a forma mais defensável de uso. Os pontos de atenção:
1. O `README.md:52` e o `glossario.md:62` dizem "evitar" marcas, mas a UI as usa. A regra escrita e a prática divergem. Ajuste a regra para "uso referencial permitido, com aviso".
2. Em produto **vendido**, a defesa da citação "sem conotação comercial" (LPI 132, IV) fica menos óbvia. A defesa do art. 132, II, pensada para acessórios, não se encaixa bem num CAD concorrente. Peça opinião ao advogado.
3. A exigência de "aprovação por escrito" dos termos do site do CAD avançado A vale para o uso das marcas **no site deles**; não decide sozinha o uso nominativo em outro produto. Ainda assim, não use logos, nem "oficial", "certificado" ou "compatível com" sem as ressalvas.

### (d) Ícones: parecidos ou copiados?

**Ícones de comando (`assets/icons/`, 356 SVGs, todos rastreados no git):**
- `scripts/gerar_icones.py` (801 linhas) gera **todos** os SVGs por código. Importa apenas `os` e `sys`; usa `math` em uma função; não faz `open` de arquivo externo, não baixa nada nem referencia pacote de ícones. Escreve em `assets/icons` via `open(...)` na linha 801.
- Nos SVGs lidos (`bool_unir.svg`), não há metadados `<metadata>`, `dc:`, `inkscape:` nem comentários de autoria; só formas geométricas e degradês.
- `scripts/gerar_icones.py:7` diz: "Estilo (referência do usuário): degradê ciano → azul, contorno azul forte, setas azul-escuro". **A origem dessa "referência do usuário" não está documentada.** Se foi um screenshot ou um conjunto de ícones de um CAD comercial, há risco de a estética ficar próxima demais. Uma paleta e um degradê não são protegidos por si, mas o conjunto de formas, sim, pode ser.
- **Risco: baixo**, com uma pergunta ao autor sobre a referência.

**Ícone do aplicativo (a fênix):**
- `assets/app/fenix-ref.png` (que abri) mostra uma ilustração em preto e branco de uma fênix sobre fundo azul com **selo "Made with AI"** no canto superior direito.
- `tools/vetorizar/src/main.rs:1-4` diz que a ferramenta "vetoriza a arte do ícone… **remove o selo do canto superior direito** e traça os contornos como caminhos SVG".
- `assets/app/fenix.svg` começa com `fill="#0b0d12" fill-rule="evenodd" d="M47.8 9.8L44.8 11.2L…"`, assinatura de contornos traçados. Isso sugere que o SVG **vigente** vem da vetorização, e não do `gerar_fenix.py`.
- `scripts/gerar_fenix.py:1-6` diz "Desenho original, gerado por código" e usa cores diferentes (amarelo/laranja/vermelho). A procedência declarada não bate com a do arquivo em uso.
- O ícone é embutido no executável (`crates/fenkra-app/build.rs:8`, `fenix.ico`) e na janela (`main.rs:60`, `fenix-256.png`).

**Riscos específicos:**
1. **Proteção autoral fraca.** O US Copyright Office (relatório de jan/2025) concluiu que material produzido **apenas por prompts** não é protegível, e o D.C. Circuit confirmou a exigência de autoria humana em *Thaler v. Perlmutter* (mar/2025; certiorari negado em mar/2026, segundo fontes secundárias). No Brasil, a Lei 9.610 define autor como pessoa física (art. 11, conforme o resumo lido); não há regra específica para IA e o tema não está resolvido. Consequência: o autor **pode não conseguir impedir** que terceiros copiem o logo, embora possa registrá-lo como marca (proteção por registro, não por autoria).
2. **Termos da ferramenta de IA.** Não sei qual foi. Termos de uso podem restringir uso comercial ou exigir a manutenção do selo. Remover um selo pode violar os termos.
3. **Semelhança.** Imagens geradas podem se parecer com obras existentes. Convém uma busca reversa de imagem.
4. **Procedência contraditória** no repositório, que atrapalha qualquer defesa futura.

**Fontes tipográficas.** `app.rs:6393-6432` lê `arial.ttf`, `calibri.ttf`, `segoeui.ttf`, `consola.ttf`, `times.ttf` (e `isocpeur.ttf`, `osifont.ttf` se existirem) **de `C:\Windows\Fonts` em tempo de execução**. Nada é embutido nem redistribuído, o que é adequado. O export SVG nomeia "Arial, Helvetica" (`export.rs:36`); o PDF referencia as fontes-base Helvetica sem embutir (`export.rs:185-186`). As fontes do egui (Hack, Ubuntu-Light, Noto Emoji) **são** embutidas no binário e exigem aviso.

**Screenshots em `docs/contextos/review/`:** são **44** JPEGs. Abri 5 (`dr-02-1`, `dr-05-1`, `dr-20-1`, `dr-30-1`, `dr-54-1`): todos eram capturas da interface do próprio Fenkra. Os 39 restantes **não foram vistos**; convém conferir todos.

### (e) Textos copiados de ajudas

**Estrutura e declarações.**
- `docs/benchmark/fontes.md:3`: "Nenhum texto foi copiado: tudo foi reescrito com palavras próprias." Lista cerca de 65 fontes, CAD avançado A (22 páginas de `help.CAD avançado A.com` e um blog), CAD avançado C (`help.fornecedor de CAD avançado.com`) e CAD avançado B via apostilas universitárias, API pública e tutoriais.
- `docs/benchmark/README.md:30`: a ajuda oficial do CAD avançado B exige login e **não foi usada**. Essa decisão reduz o risco de EULA e de termos de acesso.
- `docs/arquitetura/procedencia.md:11-13`: só **3 linhas** de registro, cobrindo "33 páginas" do benchmark e o benchmark CAD avançado B. A pesquisa avançada (`docs/pesquisa/*`, 17 arquivos) tem dezenas de URLs dentro de cada relatório (por exemplo `interface-e-arvore.md:740`), mas **nenhuma linha própria na procedência**.

**Como as amostras se comportam.** Li trechos de `docs/pesquisa/chapa.md:140-175` e de `docs/pesquisa/interface-e-arvore.md:205-225`.
- Estão em **português, em tópicos, com análise própria** ("Fenkra hoje", "Melhor prática") e etiquetas **[V]** (confirmado) e **[C]** (conhecimento geral). Isso é redação própria, não transcrição.
- Citam **nomes de comandos e rótulos de menu em inglês** (ex.: `objeto de trabalho`, `Edit > Scan or objeto de trabalho`). Nomes de comandos são curtos e funcionais, e normalmente não são protegidos por direito autoral. Não vi parágrafo copiado.
- **Limitação:** li quatro trechos de cerca de 217 mil palavras. Isso é amostragem, não verificação.

**Pontos que merecem cuidado:**
1. `docs/pesquisa/README.md:14`: "a pesquisa foi orquestrada com **workers Sonnet**… Os relatórios foram gravados como cada worker os entregou." Ou seja, **sem revisão humana** de procedência. Modelos que leem páginas e escrevem resumos podem, às vezes, ficar perto demais do original.
2. `docs/contextos/README.md:3-5`: o material foi "gerado com assistentes de IA a partir de descrições públicas dos sistemas comerciais". O `smart-fastener.md` (linhas 1-30) descreve o biblioteca de normas, o fixadores inteligentes e o assistente de furos em passo a passo com caminhos de menu ("Ferramentas > Suplementos"). Está commitado quase em bruto (`<!-- origem: Smart fastener.txt -->`). É o item de **maior proximidade** com a documentação de terceiros.
3. **Documentação da fornecedor de CAD avançado.** O resultado de busca sobre a política de documentação diz que, salvo exceções, o uso da documentação de produto exige permissão por escrito, com pré-aprovação limitada a trechos curtos com crédito. Isso reforça manter a regra "reescrever com palavras próprias" e **não** colar trechos.
4. **Direito aplicável.** A Lei 9.610/98 não protege "ideias, procedimentos normativos, sistemas, métodos, projetos" (art. 8º, I) nem "nomes e títulos isolados" (VI); o art. 46, III e VIII, permite citação e pequenos trechos em obra nova. Na UE, o TJUE (*SAS Institute v World Programming*, C-406/10) decidiu que a funcionalidade, a linguagem de programação e o formato de arquivos de dados de um programa não são protegidos pela Diretiva de Software; a corte britânica depois concluiu que copiar a funcionalidade descrita em manuais não violou direito autoral sobre o código nem os manuais, em resumo secundário. Isso reforça a conclusão de que **estudar documentação e reimplementar funcionalidade é lícito**, sem copiar texto, imagens, ícones, código ou *screenshots*.

### (f) Dados de normas e catálogos (níveis A/B/C da ADR 0008)

**Política** (`docs/arquitetura/adr/0008-biblioteca-publica.md:22-24` e `docs/biblioteca/fontes-de-dados.md:8-15`):

| Nível | O que é | Como usar |
|---|---|---|
| A | Domínio público ou licença compatível | Fonte principal, citando documento e revisão |
| B | Catálogo público gratuito do fabricante | Extrair **só medidas**, por família, citando a página; conferir amostra em outra fonte; não reproduzir o catálogo inteiro de uma vez (direito de base de dados na UE) |
| C | Norma paga (ISO, DIN, EN, ABNT, ASTM, JIS, AS/NZS, NAS) | O texto não está disponível: a norma vira só **referência**; medidas de fontes A/B em **duas fontes independentes** |
| X | Licença incompatível (GPL) ou termos desconhecidos | Só conferência; nunca fonte de cópia |

A distinção "fato x reprodução de tabela" está **correta em princípio** e coerente com a lei lida:
- A Lei 9.610, art. 7º, §2º, diz que a proteção de bases de dados **não cobre os dados em si**, e o §3º protege a forma literária ou artística, não o conteúdo técnico. Para as normas ABNT, o TRF-3 (Apelação 0010071-65.2006.4.03.6100, DJE 02/04/2014) considerou que normas técnicas ABNT não têm proteção autoral como alegado naquele caso, por serem conhecimento técnico. É uma decisão **entre as partes**, e a ABNT mantém publicamente (alerta de 2025) que as NBR são obras protegidas. O tema continua em disputa.
- O **DIN** diz, nas perguntas frequentes sobre reprodução, que copiar textos, tabelas ou imagens literalmente infringe direito autoral, que "reprodução parcial" é copiar seções específicas, e que o uso de termos, símbolos e designações normalizados é, em geral, gratuito. Não trata de bibliotecas CAD. O DIN também afirma que não autoriza nenhum uso de IA (incluindo treinamento).
- A **ISO** afirma que qualquer uso do conteúdo, inclusive cópia parcial, exige permissão escrita. Não achei orientação específica para dados dimensionais.
- Nos EUA, a questão de normas incorporadas em lei foi tratada em *Georgia v. Public.Resource.Org* (2020, doutrina dos "government edicts") e no D.C. Circuit (uso não comercial de normas incorporadas à lei é *fair use*). Na UE, o TJUE (C-588/21 P, 05/03/2024) reconheceu interesse público prevalecente para normas harmonizadas. São decisões **limitadas a normas incorporadas à lei**; não generalizam para todas as normas.

**O que está no repositório** (`biblioteca/`, 7 arquivos JSON):

| Arquivo | Conteúdo |
|---|---|
| `fixadores/iso4762.json`, `iso4017.json`, `din933.json`, `iso4032.json`, `din934.json`, `iso7089.json` | Medidas M3 a M20 (5 campos por tamanho, por exemplo `P, dk, k, s, b`), com nível `C` e licença "CC BY 4.0 (dados)" |
| `chavetas/din6885a.json` | Chaveta DIN 6885-1, nível `C` |

- As fontes listadas são sites agregadores: `fasten.it`, `tspi.at`, `opac.net`, `wermac.org`, `torqbolt.com`, `roymech.org`.
- Abri a página do `fasten.it` para ISO 4762. O rodapé diz "Reproduced by permission of DIN Deutsches Institut für Normung e. V." e traz uma tabela de dimensões ("Dimensions in mm – source DIN EN ISO 4762"). Ou seja, **o próprio agregador diz que reproduz as tabelas por permissão do DIN.** Não consegui ler os "Terms Of Service" dele.
- O volume extraído é pequeno (M3 a M20, 5 colunas, de uma tabela M1,6 a M64, com a faixa de comprimentos e o `e = s/cos30°` calculados pelo próprio código).
- **`din6885a.json`**: a primeira fonte é "tabela de referência do contexto do projeto (fonte 1, todas as colunas)", ou seja, o material **gerado por IA** de `docs/contextos`. Aqui o problema é mais de **confiabilidade** do que de PI: um dado conferido só contra texto de IA e "fonte 1" que o próprio arquivo marca como única para vários valores.
- Há valores com `confirmado_por: 1` e notas de pendência (`fontes-de-dados.md:78-88`). Isso está sinalizado, o que é boa prática.

**Avaliação.** O risco é **baixo hoje**: poucos fatos dimensionais, reexpressos num formato próprio, citando fontes. Ele **sobe** se a biblioteca crescer copiando tabelas inteiras de catálogos (Gerdau, ArcelorMittal, Aço Brasil, AISC). Para esses, os termos de uso não foram lidos:
- Gerdau: a busca não retornou os termos de uso do site nem cláusula sobre reprodução de catálogos técnicos.
- AISC: `fontes-de-dados.md:25` registra que a página de termos deu HTTP 403.
- Hi-Shear, Cherry (CherryMAX): não achei termos de uso; só os indícios de marca.

**Rótulo "CC BY 4.0 (dados)".** Os JSON se declaram CC BY 4.0. Para fatos de terceiros isso é inócuo na pior hipótese e potencialmente enganoso na pior leitura (parece reivindicar direito sobre dados de norma). Prefira "compilação do Fenkra sob CC BY 4.0; os valores dimensionais são fatos de norma, não reivindicados".

**Fixadores aeronáuticos nível A.** A política cita MS, AN, MIL-HDBK/MIL-DTL do ASSIST "com *Distribution Statement A*" (`fontes-de-dados.md:44-46`). É um critério prudente. Documentos do governo dos EUA costumam ser de uso livre, mas cada documento precisa ser conferido, e NAS é da AIA (norma paga, nível C).

**FreeCAD Fasteners.** A página do repositório `shaise/FreeCAD_FastenersWB` indica **GPL-2.0** (barra lateral e seção License do README). Uma discussão pública propõe relicenciar para LGPL-2.1, mas não consegui confirmar que foi concluída, e o README mostra GPLv2.
- `fontes-de-dados.md:38` trata como nível X ("só para conferência") e a ADR 0008 diz o mesmo. Está certo.
- Nada em `biblioteca/` ou `crates/` cita o FreeCAD Fasteners (a menção a "FreeCAD" em `crates/` é só do preset de atalhos).
- `reuso-opensource.md:36` e `:51` ainda dizem que se pode incorporar do FreeCAD, "dados do FreeCAD (verificar licença dos dados)". O **código** central do FreeCAD é LGPL-2.0+ (como afirma o ADR 0003), mas o **Fasteners Workbench é GPL-2.0**, que não combina com LGPL-2.1. Recomendo escrever isso explicitamente em `reuso-opensource.md`.

### (g) O nome "Fenkra"

**O que se fez:** duas pesquisas web (uma "extended") por "Fenkra" com software/CAD/marca/empresa, e uma por "Fenkra" + INPI/EUIPO/USPTO.
- A única ocorrência relevante foi o **próprio repositório** `JvDNasci/fenkra` ("Fenkra CAD").
- Não apareceu nenhum outro software, empresa ou registro de marca com o nome.

**Por que não conclui nada:** as bases oficiais (INPI, USPTO, EUIPO, WIPO) são consultas dinâmicas que os buscadores não indexam. A ausência de resultado **não prova** disponibilidade.

**Histórico:** o projeto se chamava **SolidCadCraft** e foi renomeado em 27/10/2026 (commit `ce3a27a`). A pasta `dist/` ainda tem 28 zips `SolidCadCraft-*`, ignorados pelo git. A busca de marca também deve cobrir esse nome se algum zip antigo foi distribuído.

**O que fazer:**
1. Pesquisar no INPI (marcas nominativas e mistas), classes 9 (software) e 42 (software como serviço, desenho/projeto técnico), e classes correlatas.
2. Repetir no USPTO, EUIPO e WIPO Global Brand Database, com variantes fonéticas (Fenkra, Phenkra, Fenkrah, Fênix + sufixos, Fenicra).
3. Checar domínio, GitHub, redes e lojas de aplicativos.
4. Considerar o depósito no INPI. A LPI, art. 129, diz que a propriedade da marca se adquire pelo **registro**; o §1º dá precedência a quem usava de boa-fé há pelo menos 6 meses. Deixar sem depositar é o risco típico de o nome ser registrado por outro.
5. Cuidado com o **símbolo da fênix**: o desenho em si (ponto 3 da matriz) não é o nome, mas um registro de marca mista deve avaliar os dois.

### Patentes

O que foi encontrado:
- **fornecedor de CAD avançado CAD avançado A Corp.**, pedido **US 2022/0253567 A1** ("Method for Suggesting Mates for a User Selected Modeled Component", US 17/585,641, depositado em 27/01/2022). Status **pendente** (sem patente concedida na página consultada). A reivindicação 1 cobre: detectar o arrasto do componente, uma **pausa** no arrasto por intervalo predeterminado, identificar superfícies dos dois componentes, compará-las e sugerir um mate.
- O Fenkra tem "Restrição automática" (`toolbar.rs:246`: "Escolha dois elementos: o tipo mais lógico é aplicado") e "Alinhamento instantâneo" (`toolbar.rs:262`). Pelo que a descrição diz, são ações por **seleção de dois elementos**, e não por arrasto e pausa. Não encontrei código de pausa no arrasto em `asm_ui.rs`. Isso **não é uma análise de infração**; é só uma observação inicial.
- **fornecedor de CAD avançado, synchronous technology:** a pesquisa trouxe material de marketing e descrição técnica, mas **não** localizei números de patente nem reivindicações. A verificação precisa de busca em Google Patents/USPTO por cessionário e termos.
- **JT:** a fornecedor de CAD avançado oferece licença de patente **sem royalties** para implementações conformes à especificação JT (patentes US 20110199382 e 8019788, segundo o documento da fornecedor de CAD avançado), limitada ao propósito de produzir, consumir e interpretar arquivos JT. O Fenkra declara JT como não implementado (`docs/pesquisa/conversor-de-arquivos.md:190`).

Quadro geral:
- A LPI, art. 10, V, exclui "programas de computador em si" de invenção. Isso **não elimina** o risco de patentes de métodos implementados por software em outros países, principalmente nos EUA.
- Nos EUA, *Alice v. CLS Bank* (2014) tornou muitas patentes de software inválidas por serem ideias abstratas implementadas em computador genérico. Mas nem todas caem, e isso só se avalia caso a caso.
- O risco de patentes **não** se elimina "reescrevendo com palavras próprias": a patente protege o método, não o texto. Só uma busca de FTO feita por profissional reduz a incerteza.

### EULAs e engenharia reversa

- O **Fenkra não faz engenharia reversa**, segundo a documentação (`visao.md` §6 e `conversor-de-arquivos.md:219`: leitura de nativos fechados é "não-objetivo"; "Nada implementado"). Isso reduz bastante o risco.
- Os EULAs obrigam **quem os aceitou**. Quem só leu a ajuda pública não aceitou EULA. Mas, se o autor usou uma instalação licenciada (própria, do trabalho ou da faculdade) para observar o comportamento, o EULA daquela instalação pode ter cláusulas sobre benchmarking, engenharia reversa e uso para desenvolver produto concorrente. **Não consegui ler o EULA do CAD avançado A** (a busca só retornou onde encontrá-lo no produto). Isso é pergunta direta ao autor e ao advogado.
- A **Lei 9.609/98**, art. 6º, III, diz que não é ofensa a semelhança decorrente de características funcionais, normas e preceitos técnicos ou forma alternativa limitada de expressão; art. 6º, IV, trata de integração a sistema. A lei **não menciona expressamente** engenharia reversa.
- Nos EUA, o *Lotus v. Borland* (1st Cir., 1995, 49 F.3d 807) considerou a hierarquia de menus um "método de operação" não protegido (§102(b)); a Suprema Corte confirmou por empate 4-4, sem precedente nacional. Em *Google v. Oracle* (2021), a Suprema Corte decidiu por *fair use* (6-2), **supondo sem decidir** a proteção da API. O quadro é "favorável, mas não pacificado" para reimplementar interfaces e atalhos.

### Formatos de arquivo

- **DXF, PDF, SVG, STEP, STL** são o que o app exporta (README e `docs/pesquisa/conversor-de-arquivos.md`). STEP/IGES são normas abertas via OCCT.
- **DWG:** a fornecedor de CAD avançado trata DWG como sua marca e diz que não impede o uso da extensão `.dwg` nem software compatível, mas veda usar "DWG" como marca sem permissão (segundo a busca; a página das diretrizes não abriu). A ODA/Teigha é paga e foi descartada para o núcleo; o LibreDWG é GPL-3.0 e só poderia entrar como processo separado (`conversor-de-arquivos.md:242`, `311-312`).
- **Nativos** (CATPart, SLDPRT, IPT): fora do escopo; a mensagem de orientação para exportar STEP é uma boa prática.

---

## 3) Checklist de conformidade para publicar e vender

### Arquivos a criar na raiz do repositório e dentro do zip

- [ ] **`LICENSE`** (ou `COPYING`) com o texto integral da LGPL-2.1, e um aviso do tipo "Fenkra é distribuído sob LGPL-2.1-or-later". Confirme a escolha com o advogado, porque o ADR 0003 ainda está como "proposta".
- [ ] **`NOTICE`** curto com:
  - nome do produto, copyright (ano e titular);
  - **aviso proeminente de que o Fenkra usa o OpenCASCADE Technology 8.0.1**, em linha com a exceção do OCCT;
  - a **oferta escrita de código-fonte** (LGPL §6c) por pelo menos 3 anos, incluindo versão exata do OCCT, link para o código upstream e a porta vcpkg, e como reconstruir;
  - o crédito do FreeType ("Portions of this software are copyright © <ano> The FreeType Project…");
  - a escolha de licença para itens duais (FTL no FreeType; Apache-2.0 no `self_cell`).
- [ ] **`THIRD-PARTY-LICENSES`** (ou `THIRD_PARTY.md` mais a pasta `licenses/`), gerado automaticamente com `cargo-about` ou `cargo-deny` e ampliado manualmente com:
  - os textos de `share/*/copyright` do vcpkg: OCCT (LGPL-2.1 e exceção), FreeType, libpng, zlib, bzip2, Brotli;
  - os textos das fontes embutidas do egui: OFL, UFL, Hack, emoji-icon-font;
  - o aviso de cada crate MIT, BSD e Apache, e os `NOTICE` upstream quando existirem.
- [ ] **`TRADEMARKS.md`** (aviso de marcas), por exemplo: "CAD avançado A e CAD avançado B são marcas da fornecedor de CAD avançado; CAD avançado C e CAD avançado J são marcas da fornecedor de CAD avançado; CAD avançado D, CAD avançado E, Blender, FreeCAD e CAD avançado I pertencem aos respectivos titulares. O Fenkra não é afiliado, patrocinado nem endossado por eles; os nomes são usados só para indicar compatibilidade de atalhos e comparação de funcionalidade." Replique o texto na tela "Sobre" do app e em `docs/README.md`.
- [ ] **`docs/biblioteca/POLITICA-DE-DADOS.md`** (política de dados de normas), consolidando ADR 0008 e `fontes-de-dados.md`, com:
  - fatos x tabelas (nunca copiar layout, ordem, notas nem figuras);
  - os níveis A/B/C/X;
  - "uma fonte por valor, duas para nível C";
  - proibição de copiar de fonte GPL;
  - o texto do rótulo de licença dos JSON.
- [ ] **`CONTRIBUTING.md`** com DCO (`Signed-off-by`) ou CLA, ver abaixo.
- [ ] **`PROCEDENCIA` ampliada** (`docs/arquitetura/procedencia.md`): uma linha por relatório de `docs/pesquisa/` e por arquivo de `docs/contextos/`.
- [ ] **`packaging/LEIA-ME.txt`** e **`packaging/TERCEIROS.txt`** (os nomes que o `release.ps1` já espera), ou, melhor, copiar `LICENSE`, `NOTICE` e `THIRD-PARTY-LICENSES` para o zip.

### Ajustes no `scripts/release.ps1`

- [ ] **Falhar** (`throw`) se `LICENSE`, `NOTICE` ou `THIRD-PARTY-LICENSES` não existirem (hoje o `Test-Path` ignora o que falta, linhas 64-68).
- [ ] Gerar `SHA256SUMS` do zip.
- [ ] Gravar no `NOTICE` do pacote a versão exata do OCCT e das libs vcpkg (a partir dos `vcpkg.spdx.json`).
- [ ] Verificar a dependência do VC++ Redistributable e, se necessário, orientar a instalação.

### CI e rotina

- [ ] Fazer o **B67** do backlog: `cargo-deny` (bloqueando GPL, AGPL e licenças sem texto), `cargo-about` e uma checagem de que cabeçalhos SPDX existem.
- [ ] Adicionar `SPDX-License-Identifier: LGPL-2.1-or-later` no topo dos arquivos `.rs`, `.cpp`, `.h`, `.inc`.
- [ ] Rodar `cargo deny` também para os alvos Linux e macOS antes de lançar para esses sistemas (hoje só o conjunto Windows foi resolvido).

### Ajustes de nomes e textos na interface

- [ ] Manter os presets referenciais, mas escrever como **"Atalhos no estilo de [marca]"**, sem ™/®, sem logos e sem a palavra "oficial".
- [ ] Incluir o **aviso de marcas** na tela "Sobre".
- [ ] Trocar "como no CAD avançado J" (`i18n.rs:1932`, `toolbar.rs:498`) por descrição neutra ("janela para a direita, cruzada para a esquerda"), se quiser reduzir citações de marca em tooltips.
- [ ] Renomear `AsmComposer`/`composer.svg` para um nome neutro (`AsmDisplay`, `exibicao.svg`) e limpar o comentário de `engrenagens.rs:1` ("estilo Design Accelerator").
- [ ] Atualizar `README.md:52` e `glossario.md:62`: "evitar" → "usar só de forma referencial; nunca como nome de comando ou recurso".
- [ ] **Nunca** nomear comandos como "assistente de furos", "fixadores inteligentes", "biblioteca de normas", "SmartMates", "Instant3D", "objeto de trabalho", "FeatureManager", "PartDesign", "TechDraw", "Hi-Lok", "CherryMAX" etc. O glossário já aponta isso; a UI atual cumpre.
- [ ] Usar "DWG" apenas como extensão de arquivo, não em nomes de produto ou marketing.

### Ícones e logo

- [ ] Identificar a **ferramenta de IA** do `fenix-ref.png`, guardar prompt, data, plano (grátis/pago) e termos de uso vigentes na época.
- [ ] Decidir: (i) refazer o logo com designer (melhor para ter titularidade clara), ou (ii) manter com a procedência registrada e aceitar proteção incerta.
- [ ] Corrigir `gerar_fenix.py:1-6`, `tools/vetorizar` e `assets/app/` para que **um só** relato de origem seja verdadeiro.
- [ ] Documentar a "referência do usuário" do estilo dos ícones (`gerar_icones.py:7`); se for captura de um CAD comercial, rever as formas.
- [ ] Conferir os 44 JPEGs de `docs/contextos/review/` (só 5 foram vistos).

### Documentação gerada por IA

- [ ] Rodar uma varredura de similaridade (frases de 8 ou mais palavras, em português e inglês) de `docs/pesquisa/*`, `docs/contextos/*` e `docs/benchmark/*` contra as páginas citadas.
- [ ] Reescrever ou remover `docs/contextos/smart-fastener.md` e trechos de `design-review.md` com descrição de comandos do CAD avançado B/CAD avançado A em prosa contínua.
- [ ] Marcar no topo dos arquivos de pesquisa: "gerado por IA, sem revisão jurídica; resumo em palavras próprias de fontes públicas listadas".

### Dados de normas e catálogos

- [ ] Adotar a política acima; reler os termos de cada fonte **antes** de importar (Gerdau, Aço Brasil, ArcelorMittal, Vallourec, AISC, Ciser etc.) e guardar uma cópia (PDF/captura datada) dos termos vigentes.
- [ ] Sempre citar: documento, página, revisão, data de acesso, por valor ou por família.
- [ ] Para nível C: duas fontes independentes, e pelo menos uma que **não** seja um agregador que reproduza tabela "por permissão do DIN/ISO".
- [ ] Trocar a fonte "contexto do projeto" do `din6885a.json` por fonte rastreável.
- [ ] Remover do `reuso-opensource.md` a sugestão de usar "dados do FreeCAD" sem ressalva; escrever que Fasteners Workbench é GPL-2.0.

### O que exigir de contribuidores

- [ ] **DCO** (`Signed-off-by`) em todo commit, **ou CLA** se o autor quiser a opção de relicenciar ou oferecer dupla licença (por exemplo, uma versão comercial). Com a LGPL-2.1-or-later como licença do projeto, o DCO sozinho **não** dá direito de relicenciar.
- [ ] Declaração de que o contribuidor é o autor e tem direito de contribuir; **sem código copiado** de GPL, de código com licença desconhecida ou de produto proprietário.
- [ ] Proibir contribuições com texto, imagens, ícones ou capturas de tela de ajudas de terceiros.
- [ ] Obrigar a declarar contribuições geradas por IA (ferramenta, termos) e proibir logos e imagens de IA sem checagem.
- [ ] Para dados de normas: nível A/B/C/X, fonte com página, sem cópia de tabelas formatadas.
- [ ] Exigir que quem tem EULA de CAD comercial **não** contribua com algo que tenha aprendido sob cláusula de confidencialidade ou engenharia reversa.

---

## 4) O que levar ao advogado (perguntas objetivas)

**Licenciamento e distribuição**
1. A LGPL-2.1-or-later é a licença certa para o Fenkra, dado que há dependências Apache-2.0 (winit, glutin etc.) e o binário é um único `.exe` estático com crates Rust? A "or-later" resolve a compatibilidade com Apache-2.0 pela LGPL-3.0? Qual licença efetiva devo declarar para o binário?
2. Em Rust, as crates são ligadas estaticamente. Como cumprir a exigência de a LGPL permitir que o usuário **modifique e relinke** o trabalho que usa a biblioteca, se alguém quiser construir em cima das crates do Fenkra? Basta publicar o código-fonte?
3. O texto de oferta de código-fonte do OCCT que vou colocar no `NOTICE` (versão 8.0.1, link upstream, porta vcpkg, instruções de build) atende a LGPL-2.1 §6 e a exceção do OCCT? Preciso hospedar eu mesmo o código-fonte do OCCT?
4. Posso **vender** o Fenkra (binário, suporte, versão paga) sob LGPL-2.1? Quais são os limites (não posso proibir redistribuição, nem usar EULA que vete engenharia reversa)? Faz sentido uma **dupla licença** com o OCCT comercial?
5. O FreeType deve ser usado sob FTL (com crédito) em vez de GPLv2? A frase de crédito precisa estar no app ou basta no `NOTICE`?
6. O MSVC runtime precisa ser tratado em avisos ou redistribuição?

**Marcas**
7. O uso de "CAD avançado A", "CAD avançado B", "CAD avançado C", "CAD avançado D", "CAD avançado E" e "Blender" como nomes de presets de atalhos, em um produto **vendido**, é uso nominativo seguro no Brasil, nos EUA e na UE? A LPI, art. 132, IV, exige "sem conotação comercial". Isso me exclui?
8. Que texto de aviso de marcas e de "não afiliação" você recomenda? Devo trocar os nomes por termos genéricos antes de comercializar?
9. Pode-se fazer **comparação** com produtos de terceiros nos documentos (benchmark) e no marketing? Quais limites?
10. Faça uma busca de anterioridade no INPI, USPTO, EUIPO e WIPO para "FENKRA" e variantes, nas classes 9 e 42. O nome está livre? Vale depositar já no INPI? Faz sentido depositar o logo (marca mista)?
11. O nome anterior (SolidCadCraft) exige alguma providência, já que houve zips distribuídos?

**Logo e imagens**
12. O logo da fênix veio de uma imagem gerada por IA, vetorizada com remoção do selo "Made with AI". Qual o risco de titularidade e de violação dos termos da ferramenta? Vale registrá-lo como marca mesmo sem direito autoral claro? Devo refazer?
13. Qual a situação da proteção autoral de arte gerada por IA no Brasil hoje (Lei 9.610, art. 11; PL 2338/2023)?

**Direito autoral, EULAs e engenharia reversa**
14. Estudar ajudas públicas (CAD avançado A, fornecedor de CAD avançado) e reescrever com palavras próprias, como faço, é seguro? As políticas de documentação da fornecedor de CAD avançado e os termos de site da fornecedor de CAD avançado limitam isso?
15. Se parte do meu conhecimento veio de **instalações licenciadas** (próprias, do trabalho ou da universidade), os EULAs impedem que eu desenvolva um produto concorrente ou publique comparações? (Preciso levar os EULAs que aceitei.)
16. Os arquivos gerados por IA em `docs/contextos` e `docs/pesquisa` podem permanecer no repositório público? Preciso reescrevê-los ou removê-los? Como documentar a procedência?
17. Atalhos de teclado, estilos de navegação e a organização de barras e comandos de outros CADs são livres para reproduzir no Brasil, nos EUA e na UE (*Lotus*, *Oracle*, *SAS v. WPL*)?

**Dados de normas e catálogos**
18. Posso incluir em uma biblioteca aberta, com licença CC BY, **medidas** de normas ISO, DIN e ABNT obtidas de catálogos de fabricantes e agregadores? Onde está a linha entre fato e tabela protegida? A frase "reproduzido por permissão do DIN" no fasten.it muda algo para quem copia depois?
19. A posição da ABNT (alerta de 2025) contra a decisão do TRF-3 (2014): qual o risco prático de usar dimensões de NBR?
20. Posso rotular os arquivos de dados como "CC BY 4.0", ou isso induz a erro por conter fatos de terceiros?
21. Quais os termos de uso de Gerdau, Aço Brasil, ArcelorMittal, AISC, Hi-Shear e Cherry para extração de medidas? Preciso pedir autorização por escrito?

**Patentes**
22. Posso contratar uma **busca de liberdade de operação (FTO)** limitada a: arrastar e sugerir mates (pedido US 2022/0253567), edição direta e *synchronous technology*, furos e fixadores inteligentes, e vistas de desenho? Em quais países (BR, EUA, UE)?
23. Meus documentos de pesquisa, que descrevem recursos de outros CADs, criam algum risco de "conhecimento" para fins de infração intencional?

**Contribuições**
24. DCO ou CLA? Qual texto você recomenda para um projeto LGPL com a possibilidade de uma versão comercial?
25. Quem é o titular do código se parte dele foi escrito com assistência de IA (copyright e termos do provedor)?

---

## 5) Fontes

**Legislação brasileira**
- [Lei 9.609/1998, Programa de computador](https://www.planalto.gov.br/ccivil_03/leis/l9609.htm) (arts. 1º-3º, 6º, 9º, 12)
- [Lei 9.610/1998, Direitos autorais](https://www.planalto.gov.br/ccivil_03/leis/l9610.htm) (arts. 7º, 8º, 46, 87)
- [Lei 9.279/1996, Propriedade industrial](https://www.planalto.gov.br/ccivil_03/leis/l9279.htm) (arts. 10, 122, 124, 125, 126, 129, 130, 132, 189, 190)

**Jurisprudência e direito comparado**
- [Lotus v. Borland (Wikipedia)](https://en.wikipedia.org/wiki/Lotus_Development_Corp._v._Borland_International,_Inc.) e [resumo do caso](https://www.studicata.com/case-briefs/case/lotus-development-corp-v-borland-international-req3070)
- [Google v. Oracle: Faegre Drinker](https://www.faegredrinker.com/en/insights/publications/2021/4/supreme-court-decides-google-llc-v-oracle-america-inc) e [Jones Day](https://www.jonesday.com/en/insights/2021/04/supreme-court-google-wins-copyright-battle)
- [SAS Institute v World Programming (SCL)](https://www.scl.org/2436-sas-v-wpl-no-copyright-says-ecj/) e [Fieldfisher](https://www.fieldfisher.com/en/insights/sas-institute-inc-v-world-programming-limited)
- [Georgia v. Public.Resource.Org e ASTM v. PRO (Husch Blackwell)](https://huschblackwell.com/newsandinsights/dc-circuit-court-holds-non-commercial-use-of-industry-standards-incorporated-into-law-is-fair-use)
- [TJUE C-588/21 P, normas harmonizadas (Verfassungsblog)](https://verfassungsblog.de/eu-harmonised-standards/) e [comunicado MoFo](https://www.mofo.com/resources/news/240305-morrison-foerster-and-fp-logue-achieve-full-reset)
- [Normas ABNT sem proteção autoral, TRF-3 (Migalhas)](https://www.migalhas.com.br/depeso/198566/justica-federal-confirma-que-normas-tecnicas-da-abnt-nao-sao-objeto-de-direitos-de-autor)
- [ABNT alerta sobre distribuição de normas (CFF)](https://site.cff.org.br/noticia/Noticias-gerais/10/09/2025/abnt-alerta-para-distribuicao-nao-autorizada-de-norma-tecnica-sobre-exame-de-urina)
- [US Copyright Office, AI Parte 2 (jan/2025)](https://copyright.gov/newsnet/2025/1060.html) e [resumo Sidley](https://datamatters.sidley.com/2025/02/07/u-s-copyright-office-issues-report-on-artificial-intelligence-and-copyrightability/)
- [Thaler v. Perlmutter, certiorari negado (Mayer Brown)](https://www.mayerbrown.com/fr/insights/publications/2026/03/supreme-court-denies-review-in-ai-authorship-case)
- [IA e direito autoral no Brasil (Migalhas)](https://www.migalhas.com.br/depeso/397458/inteligencia-artificial-e-autoria-analise-da-legislacao-internacional)
- [Alice v. CLS Bank (Cornell LII)](https://www.law.cornell.edu/supct/cert/13-298)

**Marcas, termos e EULAs**
- CAD avançado A: Legal Notices 2026
- CAD avançado A: Terms of Use (conteúdo conhecido pelo resultado de busca; a página não carregou)
- CAD avançado A: Solution Partner Graphics Charter
- fornecedor de CAD avançado: Trademarks e Guidelines for Use (conteúdo conhecido pelo resultado de busca; a página retornou 403)
- fornecedor de CAD avançado: Website Terms of Use
- fornecedor de CAD avançado e ODA, acordo sobre DWG (RTT News), [Open Design Alliance (Wikipedia)](https://en.wikipedia.org/wiki/Open_Design_Alliance)
- [Howmet: Hi-Lok](https://www.howmet.com/?p=1580) e [CHERRYMAX (Justia)](https://trademarks.justia.com/owners/sps-technologies-llc-14521)
- [CAD avançado A EULA: onde encontrar (CATI)](https://www.cati.com/blog/read-eula/)

**Patentes**
- [US 2022/0253567 A1, fornecedor de CAD avançado CAD avançado A, sugestão de mates (Google Patents)](https://patents.google.com/patent/US20220253567A1/en)
- fornecedor de CAD avançado, synchronous technology e white paper (sem números de patente)
- fornecedor de CAD avançado, JT Data Format Reference, aviso de patente; [Library of Congress, JT](https://loc.gov/preservation/digital/formats/fdd/fdd000562.shtml)

**Licenças de software e fontes**
- [LGPL-2.1 (gnu.org)](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html) (§§3 e 6)
- [OCCT: licença LGPL + exceção (Fedora)](https://fedoraproject.org/wiki/Licensing/Open_CASCADE_Technology_Public_License) e [OCCT: mudança para LGPL](https://dev.opencascade.org/content/open-cascade-technology-changes-its-license-gnu-lgpl)
- [Apache: compatibilidade com GPL](https://www.apache.org/licenses/GPL-compatibility)
- [Ubuntu Font Licence 1.0 (SPDX)](https://spdx.org/licenses/Ubuntu-font-1.0) e [SIL OFL 1.1 (SPDX)](https://spdx.org/licenses/OFL-1.1)
- [FreeCAD Fasteners Workbench (GitHub)](https://github.com/shaise/FreeCAD_FastenersWB) e [issue #307, relicenciamento](https://github.com/shaise/FreeCAD_FastenersWB/issues/307)
- [Repositório do Fenkra (GitHub)](https://github.com/JvDNasci/fenkra)

**Normas e catálogos**
- [DIN, perguntas frequentes sobre reprodução](https://www.din.de/en/about-standards/standards-and-the-law/reproduction-new/reproduction-faq)
- [ISO, aviso de copyright](https://committee.iso.org/copyright.html)
- [fasten.it, DIN EN ISO 4762](https://fasten.it/norms/norm/din_en_iso_4762)

**Fontes locais do repositório e do ambiente (consultadas em 01/11/2026)**
`Cargo.toml`, `Cargo.lock`, `crates/*/Cargo.toml`, `crates/fenkra-kernel/build.rs`, `crates/fenkra-app/src/{keymap,settings,i18n,app,toolbar,icons,asm_ui}.rs`, `scripts/{release.ps1,gerar_icones.py,gerar_fenix.py}`, `tools/vetorizar/src/main.rs`, `assets/app/*`, `biblioteca/*`, `docs/arquitetura/{adr/0001,0003,0008,procedencia,reuso-opensource}.md`, `docs/biblioteca/fontes-de-dados.md`, `docs/benchmark/{README,fontes}.md`, `docs/contextos/*`, `docs/pesquisa/*`, `docs/backlog/backlog.md`, `dist/Fenkra-0.1.0-alpha.28-windows-x64.zip`, `C:\dev\vcpkg\installed\x64-windows-release\share\*\copyright` e `vcpkg.spdx.json`, `~/.cargo/registry/.../epaint_default_fonts-0.37.0`.

---

## Limitações desta auditoria

- **Não consegui** abrir as diretrizes de marca completas da fornecedor de CAD avançado (403) nem o EULA do CAD avançado A; usei resumos de busca. A página de termos do CAD avançado A deu timeout.
- **Não consegui** consultar bases oficiais de marcas (INPI, USPTO, EUIPO), nem os termos de uso da Gerdau, do AISC, da Hi-Shear e da Cherry.
- **Não localizei** números de patente da fornecedor de CAD avançado (synchronous technology); o que cito da fornecedor de CAD avançado é um pedido pendente, não uma patente concedida.
- A licença das crates de outras plataformas (Linux, macOS) **não** foi resolvida offline; só o conjunto Windows (266 pacotes).
- A análise de textos copiados em `docs/pesquisa`, `docs/contextos` e `docs/benchmark` foi por **amostragem**, e a de screenshots, 5 de 44.
- Vários resumos de decisões judiciais vêm de fontes secundárias, não do texto da decisão. Confira os números de processo e as datas antes de citar.
- O estado "alpha.28" e o conteúdo do GitHub refletem o que estava visível em 01/11/2026.

## Handoff

**Resultado:** relatório entregue acima. Nenhum arquivo do repositório foi criado ou alterado, e não houve commit, push, release nem delegação.

**Arquivos temporários:** só no scratchpad, fora do repositório. São `meta.json`, `meta.err`, `an.py` e `lic.txt`, em `C:\Users\JOO~1\AppData\Local\Temp\claude\C--Users-Jo-o-Projetos-projeto-cad\15af042d-1deb-42e6-b062-81c4917c8c34\scratchpad\`. Servem para repetir a contagem de licenças das crates. Não escrevi o arquivo de plano do modo de planejamento, porque o pedido proibia criar arquivos.

**Checagens executadas:**
- `cargo metadata --locked --offline --filter-platform x86_64-pc-windows-msvc`, só leitura. Retornou 266 pacotes e nenhuma licença GPL-only nem AGPL; só a dupla `self_cell` e o pacote de fontes do egui merecem tratamento.
- Listagem do zip `alpha.28` (34 arquivos, sem documentos) e leitura dos `copyright` do vcpkg.
- Buscas por marcas em `crates/` e `docs/`, leitura de `fenix-ref.png` e de 5 JPEGs do review.
- Pesquisa web em leis, jurisprudência, marcas, patentes, EULAs e licenças.

**Falhas e lacunas:** as listadas em "Limitações". Os itens mais relevantes: busca de marca inconclusiva, EULAs não lidos, patentes fornecedor de CAD avançado não localizadas.

**Riscos principais:**
1. Distribuição do zip sem licença, aviso e oferta de código-fonte do OCCT.
2. Procedência e proteção do logo da fênix (imagem de IA).
3. Disponibilidade do nome "Fenkra" ainda não confirmada.

STATUS: DONE_WITH_CONCERNS
SUMMARY: Não há violação evidente de direitos por estudar outros CADs, mas faltam LICENSE/NOTICE/THIRD-PARTY e a oferta de código do OCCT no zip, o logo da fênix vem de imagem de IA, o nome "Fenkra" não teve busca de marca conclusiva, e EULAs, termos de catálogos e patentes não puderam ser verificados.
