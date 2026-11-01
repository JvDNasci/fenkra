# Síntese da pesquisa avançada

**Base:** os 15 relatórios de `docs/pesquisa/` (seções 1, 3 e 4 de cada um; seção 2 só para desempate), o [ADR 0011](../arquitetura/adr/0011-filosofia-de-modelagem.md) e o backlog B01–B63 com estado em 31/10/2026.

**Códigos de origem usados aqui e no JSON:**

| Código | Relatório |
|---|---|
| PESQ-SKT | [sketch.md](sketch.md) |
| PESQ-SOL | [solidos.md](solidos.md) |
| PESQ-SUP | [superficies.md](superficies.md) |
| PESQ-CHP | [chapa.md](chapa.md) |
| PESQ-EST | [estrutural.md](estrutural.md) |
| PESQ-MON | [montagem.md](montagem.md) |
| PESQ-DES | [desenho.md](desenho.md) |
| PESQ-PMI | [pmi.md](pmi.md) |
| PESQ-UI | [interface-e-arvore.md](interface-e-arvore.md) |
| PESQ-PRA | [prendedores-aeronauticos.md](prendedores-aeronauticos.md) |
| PESQ-PRI | [prendedores-industriais.md](prendedores-industriais.md) |
| PESQ-CNV | [conversor-de-arquivos.md](conversor-de-arquivos.md) |
| PESQ-PLM | [plm-vpm-e-save.md](plm-vpm-e-save.md) |
| PESQ-MAC | [requisitos-macro.md](requisitos-macro.md) |
| PESQ-FI | [funcoes-inteligentes.md](funcoes-inteligentes.md) |
| PESQ-TST | plano de corpus de testes (já citado no B53) |

**Colisões de prefixo nos relatórios:**
- `RF-INT` aparece em dois relatórios. Aqui ele vira `UI/RF-INT-nn` (interface e árvore) e `FI/RF-INT-nn` (funções inteligentes).
- `RF-PRD` também aparece em dois. Ele vira `PRA/RF-PRD-nn` (aeronáuticos) e `PRI/RF-PRD-nn` (industriais).

Os itens novos são numerados **N01–N45**, na mesma ordem da lista `novos` do JSON.

---

## 1. Visão macro do Fenkra e o que muda

### 1.1 O produto hoje

O Fenkra tem cerca de 58 mil linhas em Rust, 7 crates e uns 151 testes:

| Crate | Papel |
|---|---|
| `fenkra-core` | documento, árvore, sketch e solver, parâmetros, PMI, macros |
| `fenkra-kernel` | fachada C++ do OCCT 8.0 |
| `fenkra-solidos` | recompute de todos os comandos, biblioteca, perfis, fixadores, engrenagens |
| `fenkra-montagem` | solver LM de 6 GDL por corpo, juntas, BOM, preencher furos |
| `fenkra-desenho` | vistas HLR, cotas, PDF/SVG/DXF R12 |
| `fenkra-project` | versões por hash, revisões, PN |
| `fenkra-app` | interface em egui/wgpu |

**Bancadas.** Há 8 bancadas de modelagem: sketch, sólidos, superfícies/wireframe, chapa, estrutural/soldas, montagem, desenho e PMI 3D. Três serviços transversais as atendem: biblioteca de normas (fixadores, perfis, engrenagens), conversor de arquivos e projeto/PLM. As bancadas são um `enum Bench` fixo, não plugins, ao contrário do que diz o ADR 0005.

**Dados.**

| Formato | Conteúdo |
|---|---|
| `.fkp` | `Document` JSON: propriedades, parâmetros, referências, coletores, linha do tempo, PMI, links externos, `sheet` |
| `.fka` | nós que apontam para arquivos **por caminho**, restrições, juntas, padrões e features de montagem |
| `.fkd` | vistas com `Source{path}` |
| Projeto | pasta com `projeto.fenkra.json`, `pecas/` e `.versoes/<hash>` |

`FORMAT_VERSION = 1` está fixo e não há migração. Os links são caminhos de texto. A gravação não é atômica, exceto a do índice.

**Fluxos ponta a ponta hoje:**

| Fluxo | Como funciona | Problema |
|---|---|---|
| Peça → desenho | vista ligada por caminho; recarrega a cada 1,5 s | as cotas guardam pontos 3D soltos |
| Montagem → PLM | BOM em CSV plano | sem estrutura de produto |
| Estrutural → montagem → desenho | grupos por assinatura, `.fkp` + `.fka` gerados, lista de corte | comprimento pela linha de centro e numeração instável |
| Prendedores | preencher furos métricos M3–M20 | sem polegada, grip nem pontos |
| Importação → fabricação | só malha STL/OBJ; saída em DXF planificado, CSV, STEP/STL | `import_step` existe e nada o chama |

### 1.2 O que muda com a filosofia CATIA V5 + funções inteligentes

A pesquisa confirma o diagnóstico do ADR 0011: **a lacuna principal é de modelo, não de tela.** O Fenkra já tem nomes de comandos de sobra (cerca de 50 tipos de elemento de superfície e 14 comandos de chapa). Faltam quatro estruturas que o CATIA trata como plataforma:

1. **Árvore de especificação com um só objeto em trabalho.** Hoje três mecanismos fazem esse papel: `active_collector`, o marcador de retrocesso e `Session`. A árvore também precisa de contêineres ordenado e não ordenado, estado por comando e update explícito com diagnóstico.
2. **Contratos entre documentos.**
   - Publicações e interfaces com nome.
   - Links com identidade (`uid`) e estado (`->`, `->?`, `->*`, `->x`).
   - Documentos derivados (importados) com origem e hash.
3. **Referências estáveis em todo lugar**, não só em filete, chanfro e casca:
   - superfícies, chapa e estrutural;
   - cotas de desenho, PMI e restrições de montagem.

   Hoje quase todas usam ponto clicado ou polilinha copiada.
4. **Dados que não se perdem:**
   - migração de formato;
   - gravação atômica;
   - confirmação de descarte;
   - plano de salvamento e Empacotar;
   - modo pasta multiusuário seguro.

Sobre essa base entram as **funções inteligentes**: inferir, declarar, reusar e automatizar. Elas seguem o princípio de encaixe do relatório PESQ-FI:
- toda automação deixa um **objeto na árvore**;
- esse objeto usa referências v2 ou publicadas;
- ele é editável pelo mesmo assistente;
- ele tem estado de diagnóstico.

Exemplos: interfaces de encaixe, conjunto de prendedores por pontos coloridos, furo guiado por norma, furo de montagem gravado nas peças, tabela de configurações e seleção por intenção.

**Arquitetura alvo:**

```
Projeto (pasta) ── índice = cache reconstruível ── cofre por hash
   │  uid por documento; refs = uid + caminho relativo (+rev/hash)
   ├─ Peça .fkp ── árvore ordenada / não ordenada ── objeto em trabalho
   │     publicações (nome → chave dono+papel) · interfaces
   ├─ Montagem .fka ── componentes por uid · sketch/refs/publicações da montagem
   │     restrições com estado · links com estado · features com dono
   ├─ Desenho .fkd ── vistas por uid · Anchor estável em cotas/balões · estado por vista
   └─ Derivado (importado) ── origem + hash + conversor + estado
Conversor (crate + processo filho) · Biblioteca (famílias por dados, versão gravada)
```

---

## 2. Os 10 princípios de modelagem e interação

Os princípios estendem os 7 do ADR 0011. Os de número 2, 4, 5 e 7 são novos ou foram desmembrados de lá. Cada princípio traz regras concretas que fecham bugs "simples" encontrados pelos relatórios.

**P1. Referência estável ou erro visível** (ADR 1)
- Toda referência entre comandos, peças, cotas, PMI e restrições é uma chave dono + papel + assinatura (ADR 0010) ou um nome publicado. O ponto clicado vale só como reserva, com tolerância nomeada.
- Proibido no avaliador:
  - `.or(cs.last())` (superfícies, `which`);
  - escolher "a ponta mais próxima" sem tolerância (estrutural, `member_end`);
  - snap de 2 mm (PMI);
  - `refresh_refs` que mantém geometria antiga (montagem).
- Arquivos antigos têm os pontos promovidos a chaves no primeiro recompute que der certo.
- Cada comando tem um teste "apaga o pai → falhou com diagnóstico, nunca troca de alvo".

**P2. Nada ignorado em silêncio** (novo)
- Combinação inválida de parâmetros gera erro nomeado. Casos já achados:
  - saída × término "até…";
  - furo simétrico usando só a metade;
  - loft que descarta contornos internos;
  - extensão de exportação desconhecida gravando STEP;
  - série de prendedor esgotada caindo no último tamanho;
  - `let _ =` descartando validação na chapa.
- Remover cota ou restrição (aparar, filetar, apagar ponto) lista o que saiu e desfaz em um passo.
- Resultado parcial vira estado "parcial" com contagem (projeção: "N de M pontos").

**P3. Um objeto em trabalho e um contexto ativo** (ADR 2)
- Um único objeto em trabalho por peça, sublinhado na árvore e citado na barra de status. O próximo comando entra depois dele.
- Salvar com a peça retrocedida avisa e oferece "Mover para o fim".
- Na montagem, um componente ativo por vez: restrições e features só entre filhos do ativo. Editar não troca de bancada, e Voltar, Voltar ao pai e Voltar ao topo estão sempre à mão.
- Voltar à montagem só depois de salvar a peça com sucesso ou descartar as mudanças.

**P4. A árvore é especificação e o update é explícito** (CATIA, desmembrado do ADR 1)
- Contêiner ordenado (consome) × não ordenado (referencia), com regras de aninhamento. Soltar um item em lugar inválido mostra o motivo.
- Estado por comando: ok, desatualizado, falhou ou suprimido. Modo manual ou automático por documento, com "Atualizar esta feature" e Cancelar real.
- Toda falha tem `ErrorKind` estável e aponta o primeiro pai que falhou. O painel Diagnóstico oferece Realçar, Editar, Substituir, Isolar, Desativar, Reordenar e Apagar.
- Pais e filhos são consultáveis. Reordenar é validado antes de soltar.

**P5. Publicação é contrato** (ADR 1 e 3)
- Referência entre documentos só por publicação ou interface. A seleção em contexto usa só publicados por padrão.
- Remover uma publicação usada lista os dependentes. Renomear guarda um alias.
- Substituir componente religa na ordem nome publicado → interface → assinatura. Proximidade só com confirmação. O que não casou aparece numa tabela, e tudo desfaz em um passo.
- A geometria por trás de um nome publicado pode ser trocada sem quebrar os links.

**P6. Links têm identidade e estado** (ADR 3)
- Todo documento tem `uid`. A referência é `uid` + caminho relativo ao **documento** (nunca ao diretório de trabalho), mais `rev/hash` opcional.
- A resolução segue uma ordem fixa e nunca escolhe outro `uid` em silêncio.
- Estados `->`, `->?`, `->*` e `->x`, com Travar, Destravar, Quebrar e Isolar. A posição do link é recalculada no update.
- Ciclo A→B→A é recusado ao criar o link.
- Corpo importado fica embutido (BREP) e mantém vínculo com o original.

**P7. Toda automação deixa objeto editável** (PESQ-FI + ADR 4)
- Furo de montagem, conjunto de prendedores, pilha, padrão de componentes e restrição por arraste viram objetos na árvore, reabertos por "Editar com assistente".
- A feature de montagem diz onde mora: grava um furo real em cada peça, com link, ou fica só na montagem, com participantes explícitos.
- Automático é idempotente (reexecutar dá "0 novos, N pulados"). Desfaz em um passo e "Remover automáticas" não toca no que foi feito à mão.
- Inferência sem objeto persistente só vale como pré-visualização.

**P8. Nada modificado fica escondido e nada se perde** (ADR 5 + PLM)
- Gravação atômica com `.bak`. Confirmação ao abrir, criar, editar no contexto e fechar. Aviso de "mudou no disco".
- Ctrl+S salva o ativo. Ctrl+Shift+S abre o plano de salvamento, com o motivo de cada arquivo.
- Componente novo exige nome e pasta e só grava no Salvar tudo.
- Empacotar reabre o pacote para verificar.
- O formato tem `format_version` com migração testada. Versão mais nova abre só leitura.
- PN é reservado só quando a gravação dá certo.

**P9. O comando se explica** (ADR 6)
- O ícone mostra o resultado. A dica tem dois estágios, com figura entrada → operação → resultado.
- A barra de status mostra o próximo passo. Os painéis seguem suporte → modo → parâmetros → avançado.
- Há busca de comandos com sinônimos em português e inglês.
- Toda mensagem diz feature, campo e ação sugerida. Erro bruto do OCCT nunca chega ao usuário sem contexto.
- Estado nunca é só cor: sempre ícone e texto.

**P10. Toda feature tem peça de teste, e a vista nunca muda o modelo** (ADR 7)
- O corpus roda em cada commit, no CI: recompute, medidas, BRepCheck, mudar e voltar cada cota, salvar e abrir, captura visual.
- Determinismo: mesmo arquivo, mesmos números e mesma ordem de faces.
- Qualidade gráfica e pré-visualização nunca alteram volume, área, medidas ou o hash do arquivo. A malha de exibição é separada da malha de pick, medida e exportação.
- Nenhum recompute, HLR ou conversão roda na thread da interface.

---

## 3. Por bancada

Cada bancada traz o estado atual, as cinco maiores lacunas, a melhor prática adotada e o relatório de origem.

### 3.1 Sketch ([sketch.md](sketch.md))
**Estado:**
- Pontos compartilhados, solver LM, 12 restrições geométricas e 8 dimensionais.
- Só 3 estados. O solver calcula `redundant`, mas a interface nunca mostra.
- Inferência só H/V. Aparar e estender só em linhas. Projeção vira polilinha fixa.
- Seis testes. Quatro bugs prováveis por leitura: filete mantém a cota encurtada, aparar deixa ponto solto, espelhar não iguala raios, `merge_points` deixa `spline_tan` órfão.

**Lacunas:**
1. Sem estados redundante e conflito, e sem cota de referência.
2. Sem inferência nem snaps.
3. Relimitação incompleta.
4. Geometria externa sem vínculo exato.
5. Sem origem nem orientação do sketch.

**Melhor prática adotada:**
- estados e reparo como Inventor e CATIA (Sketch Analysis);
- inferência com glifo antes do clique (SW);
- conflito nunca aplica solução parcial.

### 3.2 Sólidos ([solidos.md](solidos.md))
**Estado:**
- Núcleo amplo de Part Design.
- Referências v2 já cobrem filete, chanfro, casca, saída e espessura.
- `recompute_cached` funciona.
- Problemas: saída ignorada com término "até…", furo simétrico usando metade, `CreatedBy` aplicado a todas as arestas, `material` só como texto.

**Lacunas:**
1. Términos sem lado independente, offset e início deslocado.
2. Furo sem assistente de norma.
3. Padrões que copiam a ferramenta em vez de recalcular.
4. Multicorpo raso.
5. Sem massa nem inércia.

**Melhor prática adotada:** limites por lado (CATIA LIM1/LIM2), Hole Wizard (SW) e padrão "reavaliar" (Inventor Adjust).

### 3.3 Superfícies e wireframe ([superficies.md](superficies.md))
**Estado:**
- Cerca de 50 `RefKind`. Pasta de geometria não ordenada, como o Geoset.
- Curvas são polilinhas densas, então G1 e G2 são aproximados.
- Referências fixas por ponto.
- Três escolhas silenciosas: `which` cai em "a última", `Split` ignora falhas, a projeção descarta pontos.
- Análises são só shader de tela.

**Lacunas:**
1. Escolhas silenciosas.
2. Referências por ponto.
3. Fill e Join sem sketch e sem condição por aresta.
4. Sem curva analítica.
5. Sem análises numéricas.

**Melhor prática adotada:**
- Join e Healing com relatório (CATIA);
- Knit com folga visível (SW);
- Fill sem exigir 4 lados (Onshape);
- Boundary Patch misturando sketch e arestas (Inventor).

### 3.4 Chapa metálica ([chapa.md](chapa.md))
**Estado:**
- Sólido regenerado em dois estados, com `SheetMap`.
- 14 comandos, tabela de materiais e DXF.
- Só aba e bainha registram dobras. Referências por ponto ou polilinha. O recorte atravessando a dobra usa movimento rígido.
- Isso explica DR-17, 19, 20, 22 e 54–57.

**Lacunas:**
1. Mapa de dobras parcial.
2. Aresta não nomeada.
3. Compensação espalhada.
4. Recorte que não acompanha o raio.
5. Planificação incompleta.

**Melhor prática adotada:**
- mapa de dobras como fonte única de verdade;
- regras e estilos (Inventor);
- vértice virtual (SW);
- visão dobrada e planificada concorrentes (CATIA).

### 3.5 Estrutural e soldas ([estrutural.md](estrutural.md))
**Estado:**
- Membros por linha com 10 seções e 4 modos de canto.
- Grupos por assinatura geram montagem e lista de corte (B26 feito).
- Placa e reforço apontam para ponto cru.
- Lista de corte usa a linha de centro.
- Uma linha ruim derruba o comando inteiro. Solda é só símbolo.

**Lacunas:**
1. Falha não isolada por linha.
2. Placa e reforço sem referência.
3. Comprimento de corte e ângulos ausentes.
4. Assinatura e numeração de item instáveis.
5. Sem aparar até limite nem janela de cantos.

**Melhor prática adotada:**
- falha por elemento e âncora nomeada (CATIA);
- Corner Management e comprimento de corte (SW);
- Frame Generator com troca de perfil (Inventor).

### 3.6 Montagem ([montagem.md](montagem.md))
**Estado:**
- Solver global funcional: GDL, juntas, acoplamentos, explosão, BOM, interferência.
- Restrição nunca fica "quebrada": face perdida mantém a geometria antiga.
- Edição em contexto troca de bancada.
- Subconjunto rígido e opaco.
- Furo de montagem guarda coordenadas do mundo e vira prisma de 64 lados.
- Distância 0 sem orientação.

**Lacunas:**
1. Estado e ciclo de vida de restrições e links.
2. Contexto e publicações.
3. Substituir sem relatório.
4. Features de montagem sem dono.
5. Escala do solver e diagnóstico de conflito.

**Melhor prática adotada:**
- produto ativo e publicações (CATIA);
- furo pela série e estados de link (SW);
- assistente de fixação e iMates (Inventor).

### 3.7 Desenho técnico ([desenho.md](desenho.md))
**Estado:**
- Vistas HLR, corte total, detalhe circular, planificada, cotas, símbolos, tabelas, carimbos, PDF/SVG/DXF R12.
- A associatividade é só de vista: a cota guarda pontos 3D com profundidade zero e marca o valor antigo sem avisar. Mesmo assim o catálogo diz DES-05 "completo".
- Só 1º diedro.

**Lacunas:**
1. Cotas sem âncora.
2. Sem estado, atualização nem trava de vista.
3. Sem 3º diedro nem herança de norma.
4. Vistas especiais ausentes.
5. Sem cotagem inteligente.

**Melhor prática adotada:**
- estado por vista e "update needed" (CATIA, Inventor);
- importar cotas do modelo (SW Model Items);
- herança norma → folha → vista (Solid Edge, Onshape).

### 3.8 PMI 3D ([pmi.md](pmi.md))
**Estado:**
- Anotações gráficas com âncoras no mundo e snap de 2 mm.
- Cota é distância euclidiana entre dois pontos.
- GD&T é texto: sem datum ligado a face.
- Sem estado. STEP sem PMI.
- Ainda oferece concentricidade e simetria, que a ASME 2018 removeu.

**Lacunas:**
1. Âncoras por ponto e medida ponto a ponto.
2. Sem semântica nem DRF.
3. Sem estado nem diagnóstico.
4. Sem AP242.
5. Sem planos nem vistas anotadas.

**Melhor prática adotada:**
- modelo semântico em camadas (FT&A do CATIA, MBD do Creo e do NX);
- Advisor de GD&T (CATIA);
- exportação XDE com `XCAFDoc_DimTolTool`.

### 3.9 Interface e árvore ([interface-e-arvore.md](interface-e-arvore.md))
**Estado:**
- Já existe: árvore com seções, marcador por coletor, update manual/automático com "desatualizado", assistente de reparo, menu 3D por tipo, 8 presets de atalhos, 9 estilos de navegação, dicas em dois níveis.
- A árvore clona e serializa o documento a cada quadro. O desfazer guarda 200 cópias completas.
- Duas seleções separadas: `Sel` na árvore e `picks` na viewport.

**Lacunas:**
1. Objeto em trabalho único.
2. Diagnóstico guiado com causa-raiz.
3. Pais e filhos.
4. Seleção inteligente.
5. Qualidade gráfica e desempenho da árvore.

**Melhor prática adotada:**
- objeto em trabalho e Update Diagnosis (CATIA);
- Design Doctor (Inventor);
- Select Other e barra de retrocesso (SW).

### 3.10 Biblioteca e prendedores ([prendedores-aeronauticos.md](prendedores-aeronauticos.md), [prendedores-industriais.md](prendedores-industriais.md))
**Estado:**
- 6 famílias métricas M3–M20 e um `enum` fechado de 4 tipos.
- `preencher_furos` é bom em pilha e pega, mas:
  - só entende rosca métrica;
  - não vê rebaixo nem escareado;
  - limita comprimentos a `2d–10d`;
  - cria a peça sem PN e com "Aço 1020" fixo;
  - deixa 1 GDL livre (B62).

**Lacunas:**
1. Modelo de família fechado e só métrico (sem dash nem grip).
2. Sem ponto publicado com cor = diâmetro.
3. `HoleRow` cego a assentos e tolerância.
4. Sem estrutura de junta nem assistente de substituição.
5. Dados aeronáuticos sem duas fontes abertas.

**Melhor prática adotada:**
- família como dado com gerador nomeado (Toolbox, Content Center);
- Bolted Connection editável (Inventor);
- pontos coloridos por diâmetro como diferencial do Fenkra, já que nenhum dos três CADs faz isso nativamente.

### 3.11 Conversor de arquivos ([conversor-de-arquivos.md](conversor-de-arquivos.md))
**Estado:**
- Exporta STEP básico (`STEPControl_Writer`) e STL de uma peça.
- Qualquer sufixo diferente de `.stl` vira STEP.
- Importa só STL/OBJ como malha.
- `import_step` está órfão.
- O OCCT 8.0 já oferece XDE, `DE_Wrapper`, IGES, glTF e outros, mas nada disso está ligado.

**Lacunas:**
1. Sem importação de STEP nem IGES.
2. Exportação pobre e com bug de extensão.
3. Sem módulo, relatório nem lote.
4. Sem reparo.
5. Sem vínculo com o projeto.

**Melhor prática adotada:**
- importar é operação com relatório, reparo e destino explícito;
- modos Referência × Converter (Inventor AnyCAD, SW 3D Interconnect);
- Import Diagnostics (SW).

### 3.12 PLM e salvar ([plm-vpm-e-save.md](plm-vpm-e-save.md), [requisitos-macro.md](requisitos-macro.md))
**Estado:**
- Índice JSON por pasta, versões por hash, revisões congeladas.
- Abrir outro arquivo, fechar e editar no contexto não perguntam antes de descartar.
- Escrita não atômica.
- Links por caminho.
- Contadores de ID e PN colidem em pasta compartilhada.
- O ADR 0009 promete SQLite; o código usa JSON.

**Lacunas:**
1. Perda silenciosa de dados.
2. Links sem `uid`.
3. Sem Salvar tudo, Salvar como, Empacotar nem Onde é usado.
4. Multiusuário sem trava.
5. Projeto só com peças, sem estrutura nem derivados.

**Melhor prática adotada:**
- Save Management (CATIA);
- Pack and Go (SW, Inventor);
- versão × revisão e Copy Design com reserva tardia de número (Vault);
- ciclo de vida com filhos antes do pai (3DEXPERIENCE).

---

## 4. Requisitos consolidados

Os requisitos foram deduplicados entre relatórios. **P0 e P1 aparecem por extenso**, e a coluna "→" indica o item de backlog (ver §5). Prioridade consolidada: quando relatórios divergem, vale a mais alta, salvo nota.

### 4.1 Geral, plataforma e qualidade (GER)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-GER-01 | Referência não resolvida → estado "falhou" com diagnóstico; nunca outra geometria | P0 | RNF-SOL-01, RNF-SUP-01, RNF-EST-04, RNF-PMI-01, PRA/RNF-PRD-08, FI/RNF-INT-03, RNF-PLM-06 | B15, N06 + itens por bancada |
| REQ-GER-02 | Nenhum parâmetro ou remoção ignorado em silêncio | P0 | RNF-SOL-02, RNF-CHAPA-04, RNF-SKT-05 | N11, N12 |
| REQ-GER-03 | Migração de formato por tipo; versão futura só leitura; `serde(default)` | P0 | RF-MAC-01, RNF-SKT-06, RNF-MONT-08, RNF-SUP-07, RNF-EST-05, RNF-PMI-02/07, RNF-PLM-08 | N01 |
| REQ-GER-04 | Corpus por comando com cobertura feature→teste, mudar e voltar, salvar/abrir idêntico | P0 | RNF-MAC-09, RNF-SKT-04, RNF-SOL-03/04/06, RF-SUP-47, RNF-CHAPA-02/03, RNF-PMI-04, RNF-CNV-12, RNF-PLM-14, FI/RNF-INT-07, RNF-EST-08 | B53, N09 |
| REQ-GER-05 | CI em cada commit: fmt, clippy, test, corpus | P0 | RNF-MAC-08 | N04 |
| REQ-GER-06 | LICENSE, THIRD_PARTY, cargo-deny; GPL só em processo separado | P0 | RNF-MAC-11, RNF-CNV-06 | N04 |
| REQ-GER-07 | Determinismo: mesma entrada → mesmo resultado | P0 | RNF-SKT-03, RNF-MONT-01, RNF-EST-01, FI/RNF-INT-02, RNF-CNV-04, RNF-MAC-10 | N04, N09 |
| REQ-GER-08 | Nada pesado na thread da interface; cancelável | P0 | RNF-MAC-01, RNF-DES-01, RNF-SOL-09, UI/RNF-INT-02, RNF-CNV-01 | N05, B30, N24 |
| REQ-GER-09 | Pré-visualização e qualidade gráfica nunca alteram o modelo nem as medidas | P0 | FI/RNF-INT-10, UI/RNF-INT-05, RNF-SUP-04, RNF-PMI-10 | B61 |
| REQ-GER-10 | Automação cria objeto editável pelo mesmo assistente; desfazível; não toca no manual | P0 | FI/RF-INT-01, FI/RNF-INT-04 | B58, N35, N36 |
| REQ-GER-11 | Tolerâncias derivadas do tamanho do modelo, como constantes nomeadas | P1 | RNF-SOL-07, RNF-SUP-03, RNF-EST-03 | N12, N13, N16 |
| REQ-GER-12 | Mensagens em português com campo e ação sugerida; i18n verificável | P1 | RNF-SOL-08, RNF-SUP-05, RNF-MONT-09, RNF-PLM-09, FI/RNF-INT-06, RNF-MAC-12 | B60 |
| REQ-GER-13 | Unidades reais; parâmetros texto, booleano e lista; limites; supressão por expressão | P1 | RF-MAC-18, RF-SOL-42, FI/RF-INT-35, UI/RF-INT-36 | N38 |
| REQ-GER-14 | Tabela de configurações como objeto da árvore; configuração inexistente → erro | P1 | RF-SOL-43, FI/RF-INT-31/32 | N39 |
| REQ-GER-15 | Configurações em camadas (aplicação × documento), chaves travadas, backup | P1 | UI/RF-INT-22/32, RF-MAC-41, UI/RNF-INT-06 | N37 |
| REQ-GER-16 | Templates, propriedades da empresa, projeto com caminhos de busca | P1 | RF-MAC-21/22/25/39/40 | fila futura |
| REQ-GER-17 | CLI headless estável com SemVer | P1 | RF-MAC-45 | B39 (alterado) |
| REQ-GER-18 | Linux e diretório de configuração por plataforma | P1 | RNF-MAC-07 | fila futura |
| REQ-GER-19 | Log de sessão e relatório de falha local | P1 | RNF-MAC-15 | N02 |
| REQ-GER-20 | Orçamentos de desempenho (viewport, 1.000 componentes) | P1 | RNF-MAC-03/04 | fila futura |

### 4.2 Interface e árvore (UI)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-UI-01 | Objeto em trabalho único, sublinhado; o próximo comando entra depois dele | P0 | UI/RF-INT-01, RF-MAC-05, RF-SOL-38 | B59 |
| REQ-UI-02 | Definir como objeto em trabalho, Scan, aviso ao salvar retrocedido | P1 | UI/RF-INT-02/03 | B59 |
| REQ-UI-03 | Diagnóstico com `ErrorKind`, causa-raiz e as ações | P0 | UI/RF-INT-04/05, RF-SOL-39, RF-MAC-15, RF-SUP-08, RF-CHAPA-37, RF-EST-32, RF-DES-06 | N06 |
| REQ-UI-04 | Pais e filhos com realce; Isolar referência de geometria; busca `:erro` | P1 | UI/RF-INT-07/08/34 | N06 |
| REQ-UI-05 | Update manual/automático por documento, local, Cancelar real | P1 | UI/RF-INT-09, RF-MAC-16, RF-MONT-19 | B59, N40 |
| REQ-UI-06 | Ordenado × não ordenado; reordenar validado por pais e filhos | P1 | RF-MAC-06, UI/RF-INT-28 | B59 |
| REQ-UI-07 | Desfazer nomeado; 1 comando = 1 passo; limite em MB | P0 | UI/RF-INT-11/12, UI/RNF-INT-03, RNF-MAC-05 | N05 |
| REQ-UI-08 | Árvore sem clone nem serialização por quadro | P0 | UI/RNF-INT-01 | N05 |
| REQ-UI-09 | Seleção única árvore + viewport, filtros, Selecionar outro | P1 | UI/RF-INT-13/14/15, UI/RNF-INT-09 | N32 |
| REQ-UI-10 | Laço, tangência, cadeia com regra gravada e refeita | P1 | UI/RF-INT-16, FI/RF-INT-14 | N32 |
| REQ-UI-11 | Painéis suporte → modo → parâmetros → avançado; "o que falta" | P1 | UI/RF-INT-18, RNF-SUP-08, RNF-CHAPA-07 | B20 |
| REQ-UI-12 | Atalhos por id estável; busca de comandos PT/EN | P1 | UI/RF-INT-21/22, FI/RF-INT-39 | B60 |
| REQ-UI-13 | Dicas em dois estágios com figura | P1 | UI/RF-INT-24 | B60 |
| REQ-UI-14 | Câmera: pan proporcional, zoom ao cursor, ajustar à seleção | P1 | UI/RF-INT-29 | B17 |
| REQ-UI-15 | Qualidade gráfica por documento; malha de exibição separada | P0 | UI/RF-INT-30/31 | B61 |

### 4.3 Sketch (SKT)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-SKT-01 | Estados sub, total, redundante, conflito e vínculo perdido com cor e texto | P0 | RF-SKT-01 | N11 |
| REQ-SKT-02 | Cota de referência; redundante vira referência | P0 | RF-SKT-02, FI/RF-INT-06 | N11 |
| REQ-SKT-03 | Conflito mantém a última geometria válida; falha não corrompe | P0 | RF-SKT-03, RNF-SKT-08 | N11 |
| REQ-SKT-04 | Correções: filete/chanfro, aparar/estender, espelhar, `merge_points`, `PointOnCurve` | P0 | RF-SKT-04–08 | N11 |
| REQ-SKT-05 | Tolerância de coincidência única (0,001 mm) | P0 | RNF-SKT-01 | N11 |
| REQ-SKT-06 | Remoção de cota ou restrição listada e desfazível | P0 | RNF-SKT-05 | N11 |
| REQ-SKT-07 | Coincidência explícita removível | P1 | RF-SKT-09 | N11 |
| REQ-SKT-08 | Inferência e snaps priorizados, glifo antes do clique, sem contradição | P1 | RF-SKT-10, FI/RF-INT-02/03/04 | N33 |
| REQ-SKT-09 | Cota inteligente 2.0, nomes `d1…`, Completar sketch | P1 | RF-SKT-11/12/13, FI/RF-INT-05 | fila futura |
| REQ-SKT-10 | Analisar e reparar sketch | P1 | RF-SKT-14, FI/RF-INT-07 | N33 |
| REQ-SKT-11 | Relimitação completa em todas as curvas | P1 | RF-SKT-15/16/17 | B18 |
| REQ-SKT-12 | Offset de cadeia com relação; transformações que mantêm restrições | P1 | RF-SKT-18/19 | B32 |
| REQ-SKT-13 | Projeção exata com vínculo e estado; interseção com todos os pontos | P1 | RF-SKT-21/22/23 | N34 |
| REQ-SKT-14 | Origem e orientação do sketch | P1 | RF-SKT-24 | N34 |
| REQ-SKT-15 | Solver ≤ 16 ms por arrasto (200 pontos) | P1 | RNF-SKT-02, FI/RNF-INT-01 | fila futura |

### 4.4 Sólidos (SOL)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-SOL-01 | Término por lado, offset, início deslocado, "até o último" real | P0 | RF-SOL-01/02/03/05 | N26 |
| REQ-SOL-02 | Saída, furo simétrico e `CreatedBy` sem perda silenciosa | P0 | RF-SOL-04/10/20 | N12 |
| REQ-SOL-03 | Furo guiado por norma; "personalizado"; tipos; posição associativa | P0 | RF-SOL-08/09/12, FI/RF-INT-10 | N27, B48 |
| REQ-SOL-04 | Rosca cosmética como atributo; rosca parcial com preview | P1 | RF-SOL-13/14 | N27, B05 |
| REQ-SOL-05 | Casca robusta com fallback e faces ignoradas listadas | P0 | RF-SOL-25 | B06 |
| REQ-SOL-06 | Espessura em face com ignorar faces | P1 | RF-SOL-26 | B07 |
| REQ-SOL-07 | Padrão "cópia exata" × "reavaliar por instância" | P0 | RF-SOL-30 | N28 |
| REQ-SOL-08 | Material com densidade, massa e inércia; carimbo ligado | P0 | RF-SOL-45/46, RF-DES-45 | N29 |
| REQ-SOL-09 | Contornos selecionados e parede fina unificada | P1 | RF-SOL-06/07 | fila futura |
| REQ-SOL-10 | Nervura paralela/normal com arcos | P1 | RF-SOL-18 | B11 |
| REQ-SOL-11 | Aviso de contornos descartados; revolução validada | P1 | RF-SOL-17/47 | N12 |
| REQ-SOL-12 | Chaves v2 fase 2; papéis em toda feature nova | P1 | RF-SOL-22, RNF-SOL-12 | B15 |
| REQ-SOL-13 | Eixo de primeira classe; `Fixed` vira associativo; plano de face falha visível | P1 | RF-SOL-35/36/37 | B13 |
| REQ-SOL-14 | Multicorpo (dividir mantendo os lados, Union Trim, participantes) | P1 | RF-SOL-27/28/29 | fila futura |
| REQ-SOL-15 | Recompute incremental e sólido válido após cada feature | P1 | RNF-SOL-05/10, RNF-CHAPA-09 | N09 |

### 4.5 Superfícies (SUP)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-SUP-01 | Fim das escolhas silenciosas (`which`, Split, projeção) | P0 | RF-SUP-02/03/05 | N13 |
| REQ-SUP-02 | Face e aresta por dono + papel | P0 | RF-SUP-07 | N13 |
| REQ-SUP-03 | Estado por elemento com as ações de diagnóstico | P0 | RF-SUP-08 | N06 |
| REQ-SUP-04 | Fill/Join com sketch, cadeia, condição por aresta; relatório de Join | P0 | RF-SUP-13/21/22 | B09 |
| REQ-SUP-05 | Extract/Boundary refazem a propagação e informam a contagem | P0 | RF-SUP-25/26 | B21 |
| REQ-SUP-06 | Curva analítica (círculo/NURBS) preservada | P1 | RF-SUP-01/33 | fila futura |
| REQ-SUP-07 | "Até elemento" e limites 1/2 | P1 | RF-SUP-11/12/29 | B14 |
| REQ-SUP-08 | Loft com guias, spine, acoplamento e condição de ponta | P1 | RF-SUP-15/16 | B12, B34 |
| REQ-SUP-09 | Curvatura no kernel; connect checker numérico | P1 | RF-SUP-39/43 | B34 |
| REQ-SUP-10 | Trim por regiões; sweep fixo com direção; curva paralela completa | P1 | RF-SUP-04/19/24/32 | fila futura |

### 4.6 Chapa (CHP)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-CHP-01 | Mapa de dobras geral | P0 | RF-CHAPA-12 | N14 |
| REQ-CHP-02 | Compensação única | P0 | RF-CHAPA-03 | N14 |
| REQ-CHP-03 | Aba por aresta nomeada; tangência; espessura e vizinhas validadas | P0 | RF-CHAPA-08/11/13/15 | N15, B01 |
| REQ-CHP-04 | Aba por superfície viva, com falha explícita | P0 | RF-CHAPA-17 | B04 |
| REQ-CHP-05 | Dobra por sketch registrada no mapa | P0 | RF-CHAPA-21 | B03 |
| REQ-CHP-06 | Recorte através da dobra por mapeamento do perfil | P0 | RF-CHAPA-23 | B02 |
| REQ-CHP-07 | Planificação exata com Check Overlapping | P0 | RF-CHAPA-32 | N23 |
| REQ-CHP-08 | Regra de chapa nomeada; raio mínimo verificado | P1 | RF-CHAPA-01/02, FI/RF-INT-17 | fila futura |
| REQ-CHP-09 | Posição da dobra, referência de comprimento, extensão lateral | P1 | RF-CHAPA-09/10 | B31 |
| REQ-CHP-10 | DXF com camadas; tabela de dobras ordenada; etiquetas iguais na vista | P1 | RF-CHAPA-33/34, RF-DES-21/49 | N23 |
| REQ-CHP-11 | Cantos e meia-esquadria fora de 90° | P1 | RF-CHAPA-14/24 | fila futura |
| REQ-CHP-12 | Bainha, joggle, perfil na aresta, parede aberta | P1 | RF-CHAPA-06/16/19/20 | fila futura |
| REQ-CHP-13 | Desdobrar/redobrar parcial | P1 | RF-CHAPA-22 | B35 |
| REQ-CHP-14 | Propriedades automáticas; estampo só no seu estado | P1 | RF-CHAPA-28/36 | N23 |

### 4.7 Estrutural (EST)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-EST-01 | Falha isolada por linha | P0 | RF-EST-01 | N16 |
| REQ-EST-02 | Placa e reforço por membro + extremidade | P0 | RF-EST-02 | N16 |
| REQ-EST-03 | Comprimento de corte e ângulos de ponta | P0 | RF-EST-03 | N17 |
| REQ-EST-04 | Assinatura por congruência; item e nomes estáveis | P0 | RF-EST-04/05/29 | N17 |
| REQ-EST-05 | Placas e reforços na lista e nos balões; propriedades e atributo de tipo | P1 | RF-EST-06/25/41 | N17 |
| REQ-EST-06 | Aparar até limite; canto rente; distância pelo sólido; janela de cantos | P1 | RF-EST-07–10/12 | B37 |
| REQ-EST-07 | Regra de orientação pública; prolongar só nas pontas livres | P1 | RF-EST-13/14/42 | fila futura |
| REQ-EST-08 | Perfis completos com massa ±1% e fonte | P1 | RF-EST-16, RNF-EST-09/10 | B27 |
| REQ-EST-09 | Esqueleto publicado; tampa e reforço avançados | P1 | RF-EST-18/20/21 | fila futura |
| REQ-EST-10 | Símbolo derivado do cordão; plano de corte 1D | P1 | RF-EST-23/26 | fila futura |
| REQ-EST-11 | Sub-soldagem, montagem viva, desenho por item | P1 | RF-EST-27/28/30, RF-DES-43 | B50 |

### 4.8 Montagem (MON)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-MON-01 | Estado de restrição; nada se move com referência perdida; reconectar | P0 | RF-MONT-01/02, FI/RF-INT-30 | N08 |
| REQ-MON-02 | Orientação, distância 0, pares válidos; `AlignOnce` reverte | P0 | RF-MONT-05/06/07, FI/RF-INT-22 | N08 |
| REQ-MON-03 | Substituir com escopo, relatório e religação por nome | P0 | RF-MONT-03/04, RF-PLM-16, RF-MAC-14, FI/RF-INT-25, PRA/RF-PRD-27 | B55 |
| REQ-MON-04 | Edição em contexto; voltar só se salvar; sem arquivo órfão | P0 | RF-MONT-08, RF-MAC-07, RF-PLM-42/43 | B54 |
| REQ-MON-05 | Links com estado, posição viva, política e ciclo | P0 | RF-MONT-09/10, RF-MAC-10/11/12, RF-PLM-47 | N07 |
| REQ-MON-06 | Furo de montagem com dono e cilindro exato | P0 | RF-MONT-11/12, FI/RF-INT-26 | B57 |
| REQ-MON-07 | Sketch, referências e publicações na montagem; esqueleto | P0 | RF-MONT-13, RF-MAC-08/09 | B55 |
| REQ-MON-08 | Extração sem perda; nada descartado sem relatório | P0 | RF-MONT-17, RNF-MONT-02 | N10 |
| REQ-MON-09 | Produto ativo | P1 | RF-MONT-14 | B54 |
| REQ-MON-10 | Conflito com conjunto mínimo; aglomerados; GDL livres | P1 | RF-MONT-18/20, RNF-MONT-04/05/07 | N40 |
| REQ-MON-11 | Limites e restrições avançadas | P1 | RF-MONT-21/22 | B36 |
| REQ-MON-12 | Interfaces de encaixe; restrição por arraste* | P1 | FI/RF-INT-20/21/23/24 | N36 |
| REQ-MON-13 | Subconjunto flexível; padrão por fonte; espelhar; interferência | P1 | RF-MONT-15/16/23/24/25 | fila futura |

\* O relatório PESQ-FI marca interfaces como P0. Aqui ficam P1 porque as publicações (B55) já dão o contrato P0, e as interfaces são a camada de conveniência por cima.

### 4.9 Biblioteca e prendedores (FIX)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-FIX-01 | Família por dados; designação livre; polegada, dash, grip; comprimentos por família | P0 | PRI/RF-PRD-01/02/03, PRA/RF-PRD-01/02 | B42 |
| REQ-FIX-02 | PN, material pela classe, massa; versão da biblioteca nas peças | P0 | PRI/RF-PRD-04/05/24 | B42 |
| REQ-FIX-03 | Ponto publicado com vetor, diâmetro, cor e rótulo; normal pela superfície | P0 | PRA/RF-PRD-08–11 | B58 |
| REQ-FIX-04 | Junta, preencher por cor, grip pelas faces, dash pela faixa; idempotente | P0 | PRA/RF-PRD-12–15, PRA/RNF-PRD-02/07 | B58 |
| REQ-FIX-05 | GDL = 0 em todo prendedor | P0 | PRA/RF-PRD-18 | B62 |
| REQ-FIX-06 | `HoleRow` com assentos e tolerância; furo aeronáutico reconhecido | P0 | PRI/RF-PRD-17, PRA/RF-PRD-16/17 | N22 |
| REQ-FIX-07 | Dados em 2 fontes, com licença; todos os tamanhos gerados no CI | P0 | PRI/RNF-PRD-01/02/04, PRA/RNF-PRD-03/04 | B42, B63 |
| REQ-FIX-08 | Assistente, modo manual, pilha recalculável | P1 | PRA/RF-PRD-19–22, PRI/RF-PRD-18/19, FI/RF-INT-27/28 | N35 |
| REQ-FIX-09 | Famílias aeronáuticas e industriais adicionais | P1 | PRA/RF-PRD-04/06/07, PRI/RF-PRD-06–12 | B63, B51 |
| REQ-FIX-10 | Publicações do fixador; equivalência pol↔mm explícita | P1 | PRI/RF-PRD-21, PRA/RF-PRD-24 | B42, N35 |

### 4.10 Desenho (DES)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-DES-01 | Cota e balão com âncora estável; quebrada = vermelha; Ø analítico | P0 | RF-DES-01/02/24/37 | N18 |
| REQ-DES-02 | Estado por vista, Atualizar, Travar/Isolar, sem update silencioso | P0 | RF-DES-03–06, RNF-DES-02, RF-MAC-42 | B30 |
| REQ-DES-03 | 1º/3º diedro; vista filha pelos eixos da mãe | P0 | RF-DES-07/08 | B30 |
| REQ-DES-04 | Valor da cota = modelo; salvar/abrir idêntico | P0 | RNF-DES-03/04 | N18, N09 |
| REQ-DES-05 | Cortes completos, auxiliar, breakout, interrompida, detalhe refinado | P1 | RF-DES-10/11/13–16/18 | N43 |
| REQ-DES-06 | Cotagem inteligente, importar cotas, conjuntos, chamadas, centros | P1 | RF-DES-23/26/28/30/32 | N44 |
| REQ-DES-07 | Norma com herança, hachura por material, fundo de folha | P1 | RF-DES-09/33/44 | fila futura |
| REQ-DES-08 | BOM configurável, tabela de furos por vista, revisões | P1 | RF-DES-36/39/41 | fila futura |
| REQ-DES-09 | DXF R2000+ com entidades nativas | P1 | RF-DES-48 | N42 |
| REQ-DES-10 | Vista a partir de captura PMI (MBD → desenho) | P1 | RF-DES-46 | fila futura |
| REQ-DES-11 | Desfazer cobre Atualizar; escala 1:1 no PDF | P1 | RNF-DES-07/10 | B30 |

### 4.11 PMI
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-PMI-01 | Medida pelo alvo (face-face, Ø de furo); chaves; sem snap de 2 mm | P0 | RF-PMI-01/02/04/05 | N19 |
| REQ-PMI-02 | Status por anotação com diagnóstico | P0 | RF-PMI-03/07 | N20 |
| REQ-PMI-03 | Componente-dono em montagem; anotação segue o componente | P0 | RF-PMI-06/38 | N20 |
| REQ-PMI-04 | Âncoras antigas promovidas | P0 | RNF-PMI-02 | N19 |
| REQ-PMI-05 | Modos de cota, precisão, tolerância tipada | P1 | RF-PMI-08/09/10 | N45 |
| REQ-PMI-06 | Datum/DRF, advisor, matriz de validade, norma ativa, símbolos vetoriais, validação | P1 | RF-PMI-13/14/16–19/21/22/33 | N45 |
| REQ-PMI-07 | Planos de anotação e vistas anotadas | P1 | RF-PMI-26/27/28 | fila futura |
| REQ-PMI-08 | Várias chamadas e realce | P1 | RF-PMI-31 | B23 |
| REQ-PMI-09 | Apagar confirmável e desfazível | P1 | RF-PMI-44, RNF-PMI-11 | N20 |

### 4.12 Conversor (CNV)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-CNV-01 | Exportar valida a extensão; caminhos com acento | P0 | RF-CNV-02/39 | N21 |
| REQ-CNV-02 | Importar STEP (XDE) com unidades e relatório; corpo embutido | P0 | RF-CNV-03/06/28, RF-MAC-35 | N24, N25 |
| REQ-CNV-03 | Exportar STEP via XDE com nomes, cores e estrutura | P0 | RF-CNV-10 | N24 |
| REQ-CNV-04 | Falha do OCCT isolada; ida e volta em 1e-6 | P0 | RNF-CNV-01/05 | N24 |
| REQ-CNV-05 | Menus Importar/Exportar separados; diálogo com destino | P1 | RF-CNV-01/05/25 | N25 |
| REQ-CNV-06 | Relatório, reparo em níveis, IGES | P1 | RF-CNV-07/09/13, RF-MAC-36 | N41 |
| REQ-CNV-07 | Montagem neutra com instâncias; DXF ler/gravar | P1 | RF-CNV-11/18/26 | N42 |
| REQ-CNV-08 | Vínculo com o original, derivados no projeto, lote | P1 | RF-CNV-27/29/30/32, RF-PLM-40 | fila futura |

### 4.13 PLM e salvar (PLM)
| ID | Requisito | P | Origem | → |
|---|---|---|---|---|
| REQ-PLM-01 | Confirmar antes de descartar (4 gatilhos) | P0 | RF-PLM-01 | N02 |
| REQ-PLM-02 | Gravação atômica + `.bak`; mudança externa; autorrecuperação | P0 | RF-PLM-02/03/17, RF-MAC-02, RNF-MAC-06, RNF-PLM-01, RNF-MONT-03 | N02 |
| REQ-PLM-03 | Plano de salvamento com motivos; atalhos iguais nas 3 bancadas | P0 | RF-PLM-04/05, RF-MAC-34 | B56 |
| REQ-PLM-04 | `uid` + caminho relativo ao documento; resolução determinística | P0 | RF-PLM-08/09/10, RF-MAC-04 | N03 |
| REQ-PLM-05 | Modo pasta seguro; contadores por reserva; sem SQLite compartilhado | P0 | RF-PLM-20/21, RNF-PLM-03 | B25 |
| REQ-PLM-06 | PN com prévia sem consumo; imutável após revisão | P0 | RF-PLM-25/37 | B47 |
| REQ-PLM-07 | Empacotar com verificação | P0 | RF-MAC-26, RF-PLM-14/15 | N30 |
| REQ-PLM-08 | Salvar como em 4 modos com tabela de referências | P1 | RF-PLM-06/07, RF-MAC-27 | N30 |
| REQ-PLM-09 | Relações, onde é usado, reparar links, renomear/mover transacional | P1 | RF-PLM-11/12/13/33, RF-MAC-13/28 | N31 |
| REQ-PLM-10 | Check-out/check-in com trava (TTL) e hash base | P1 | RF-PLM-18/19, RF-MAC-30 | B25 |
| REQ-PLM-11 | Projeto com todos os tipos e estrutura derivada; item por `uid` | P1 | RF-PLM-29/30/31/45, RF-MAC-23 | B25 |
| REQ-PLM-12 | Regras de PN, unicidade normalizada, versão com autor | P1 | RF-PLM-23/36/38, RF-MAC-33 | B47 |
| REQ-PLM-13 | Ciclo de vida configurável; liberação de filhos antes do pai | P1 | RF-PLM-26/27/28, RF-MAC-31 | fila futura |

**Totais consolidados:**
- 67 requisitos P0 e 81 P1, a partir de cerca de 560 RF/RNF originais, já sem duplicatas.
- 19 requisitos P1 ficam na "fila futura": viáveis, mas fora do teto de 45 itens novos.

**P2/P3 por área** (contagem bruta, antes de deduplicar):

| Área | P2 | P3 |
|---|---|---|
| Sketch | 8 | 2 |
| Sólidos | 15 | 1 |
| Superfícies | 20 | 4 |
| Chapa | 12 | 2 |
| Estrutural | 15 | 5 |
| Montagem | 14 | 7 |
| Desenho | 16 | 2 |
| PMI | 17 | 7 |
| Interface/árvore | 11 | 4 |
| Prendedores (aero + industrial) | 18 | 1 |
| Conversor | 16 | 5 |
| PLM | 12 | 2 |
| Macro/plataforma | 17 | 2 |
| Inteligentes (transversal) | 13 | 2 |

---

## 5. Matriz de rastreabilidade (P0/P1 → backlog)

A matriz está agrupada por item, para conferência nos dois sentidos. "alt." indica item existente com texto ampliado no JSON.

**Onda 0**

| Item | Requisitos |
|---|---|
| B53 (alt.) | GER-04 |
| B15 (alt., vai para a onda 0) | GER-01, SOL-12 |
| B54 (alt.) | MON-04, MON-09 |
| B55 (alt.) | MON-03, MON-07 |
| B56 (alt.) | PLM-03 |
| B57 (alt.) | MON-06 |
| B58 (alt.) | FIX-03, FIX-04, GER-10 |
| B59 (alt.) | UI-01, UI-02, UI-05, UI-06 |
| B60 (alt.) | UI-12, UI-13, GER-12 |
| B61 (alt., sobe para P0) | UI-15, GER-09 |
| B62 | FIX-05 |
| B42 (alt., vai para a onda 0) | FIX-01, FIX-02, FIX-07, FIX-10 |
| N01 | GER-03 |
| N02 | PLM-01, PLM-02, GER-19 |
| N03 | PLM-04 |
| N04 | GER-05, GER-06, GER-07 |
| N05 | UI-07, UI-08, GER-08 |
| N06 | UI-03, UI-04, SUP-03, GER-01 |
| N07 | MON-05 |
| N08 | MON-01, MON-02 |
| N09 | GER-04, GER-07, SOL-15, DES-04 |

**Onda 1**

| Item | Requisitos |
|---|---|
| B01–B04 (alt.) | CHP-03 a CHP-06 |
| B06 (alt.) | SOL-05 |
| B07 | SOL-06 |
| B09 (alt.) | SUP-04 |
| B11 (alt.) | SOL-10 |
| B13 (alt.) | SOL-13 |
| B14 (alt.) | SUP-07 |
| B21 (alt., sobe para P0) | SUP-05 |
| B30 (alt., vai para a onda 1 como P0) | DES-02, DES-03, DES-11, GER-08 |
| B05 | SOL-04 |
| N10 | MON-08 |
| N11 | SKT-01 a SKT-07, GER-02 |
| N12 | SOL-02, SOL-11, GER-02, GER-11 |
| N13 | SUP-01, SUP-02, GER-11 |
| N14 | CHP-01, CHP-02 |
| N15 | CHP-03 |
| N16 | EST-01, EST-02, GER-11 |
| N17 | EST-03, EST-04, EST-05 |
| N18 | DES-01, DES-04 |
| N19 | PMI-01, PMI-04 |
| N20 | PMI-02, PMI-03, PMI-09 |
| N21 | CNV-01 |
| N22 | FIX-06 |

**Onda 2**

| Item | Requisitos |
|---|---|
| B17 (alt.) | UI-14 |
| B18 (alt.) | SKT-11 |
| B20 (alt.) | UI-11 |
| B23 | PMI-08 |
| B25 (alt., sobe para P0) | PLM-05, PLM-10, PLM-11 |
| B47 (alt.) | PLM-06, PLM-12 |
| B48 | SOL-03 (parte) |
| N23 | CHP-07, CHP-10, CHP-14 |
| N24 | CNV-02, CNV-03, CNV-04, GER-08 |
| N25 | CNV-02, CNV-05 |
| N26 | SOL-01 |
| N27 | SOL-03, SOL-04 |
| N28 | SOL-07 |
| N29 | SOL-08 |
| N30 | PLM-07, PLM-08 |
| N31 | PLM-09 |
| N32 | UI-09, UI-10 |
| N33 | SKT-08, SKT-10 |
| N34 | SKT-13, SKT-14 |
| N35 | FIX-08, FIX-10, GER-10 |
| N36 | MON-12, GER-10 |
| N37 | GER-15 |
| N38 | GER-13 |

**Onda 3**

| Item | Requisitos |
|---|---|
| B12 | SUP-08 |
| B27 (alt.) | EST-08 |
| B31 (alt.) | CHP-09 |
| B32 (alt., sobe para P1) | SKT-12 |
| B34 (alt., sobe para P1) | SUP-08, SUP-09 |
| B35 | CHP-13 |
| B36 (alt., sobe para P1) | MON-11 |
| B37 (alt., sobe para P1) | EST-06 |
| B39 (alt., sobe para P1) | GER-17 |
| B50 (alt.) | EST-11 |
| B51 (alt.), B63 (alt.) | FIX-09 |
| N39 | GER-14 |
| N40 | MON-10, UI-05 (montagem) |
| N41 | CNV-06 |
| N42 | CNV-07, DES-09 |
| N43 | DES-05 |
| N44 | DES-06 |
| N45 | PMI-05, PMI-06 |

**Fila futura** (P1 sem item; viram itens quando houver espaço):

| Área | Requisitos |
|---|---|
| Geral | GER-16 (templates e caminhos), GER-18 (Linux), GER-20 (desempenho de montagens grandes) |
| Sketch | SKT-09 (cota inteligente 2.0 e Completar sketch), SKT-15 (solver rápido) |
| Sólidos | SOL-09 (multi-pad e parede fina), SOL-14 (multicorpo) |
| Superfícies | SUP-06 (curva analítica), SUP-10 |
| Chapa | CHP-08 (regra de chapa), CHP-11, CHP-12 |
| Estrutural | EST-07, EST-09, EST-10 |
| Montagem | MON-13 |
| Desenho | DES-07, DES-08, DES-10 |
| PMI | PMI-07 |
| Conversor | CNV-08 |
| PLM | PLM-13 |
| Inteligentes | Power Copy / modelos de recurso (FI/RF-INT-33/34) |

**Nenhum P0 ficou sem item.**

---

## 6. Roteiro recomendado

**Volume:**
- Itens novos: ≈ 13,98 M tokens (onda 0: 2,45 M; onda 1: 3,78 M; onda 2: 4,90 M; onda 3: 2,85 M).
- As ampliações de texto pedem revisar as estimativas destes itens:
  - B15: +100 k;
  - B25: +150 k;
  - B30: +150 k (≈ 450 k);
  - B37: +250 k (≈ 400 k);
  - B42: +150 k (≈ 300 k);
  - B34: +50 k.
- Com o backlog atual (11,8 M), o total fica em ≈ 26,7 M.

A onda 0 cresce para ≈ 6,2 M e por isso é dividida em sub-ondas, cada uma liberável como versão alpha.

### Onda 0: fundação (de `alpha.28` até cerca de `alpha.32`)

**0.1 Rede de segurança**
- N01 migração de formato. Tem de vir **antes** de B54/B55, que mudam `Assembly`.
- N02 gravação segura.
- N04 CI e licenças.
- B53 corpus, depois N09 corpus por comando.
- N05 documento sem clone e desfazer nomeado.

Sem dependências externas. É o que torna as mudanças seguintes testáveis.

**0.2 Árvore**
- B59 objeto em trabalho e update.
- N06 diagnóstico e pais/filhos, que depende de N05 para não pesar a cada quadro.
- B15 fase 2 (referências v2) em paralelo, porque superfícies, chapa, PMI e desenho dependem dela.

**0.3 Identidade e links**
- N03 `uid` e resolução, depois N07 links com estado, depois B56 plano de salvamento.
- B56 depende de N02 e N03.

**0.4 Contexto de montagem**
- N08 estado das restrições, depois B55 publicações, depois B54 edição em contexto, depois B57 furo entre peças.
- Na trilha dos prendedores: B42 modelo genérico, depois B58 pontos coloridos e junta, depois B62. B58 depende de B55 (ponto publicado) e B57 (furo nas peças).

**0.5 Mostrar e explicar**
- B60 e B61, em paralelo, a qualquer momento.

### Onda 1: erros silenciosos e bugs do design review (`alpha.33`)

**Regra de entrada:** cada item começa reproduzindo o bug provável com um teste que falha. Os relatórios inferiram esses bugs pela leitura do código, sem executar.

| Trilha | Ordem |
|---|---|
| Sketch | N11 |
| Sólidos | N12 → B05, B06, B07, B11, B13 |
| Superfícies | N13 (depende de B15) → B09, B21, B14, B10 |
| Chapa | N14 → N15 → B01–B04 |
| Estrutural | N16 → N17 |
| Desenho e PMI | N18 (cria o `Anchor` compartilhado) → N19 → N20; B30 em paralelo com N18 |
| Montagem | N10 |
| Rápidos | N21, N22 (depende de B57); B08, B12 |

### Onda 2: profundidade e funções inteligentes (`alpha.34`)

| Trilha | Ordem |
|---|---|
| Sólidos | N26 → N27 (depende de B48 e N22) → N28; N29 |
| Conversor | N24 → N25 |
| PLM | B25 (modo pasta seguro) → N30, N31; B47 |
| Interação | N32 (depois do B21); N33 e N34 (depois de N11); B17–B20 |
| Montagem inteligente | N35 (depende de B58); N36 (depende de B55 e N08) |
| Plataforma | N37, N38 |
| Chapa | N23 (depende de N14) |

### Onda 3: módulos e paridade (`alpha.35+`)
- N39 (depende de N38).
- N40 (solver por aglomerados).
- N41 e N42 (dependem de N24 e N25).
- N43 e N44 (dependem de N18 e B30).
- N45 (depende de N19 e N20).
- Os itens de onda 3 já existentes, com B37, B39 e B50 ampliados.
- Em seguida, a fila futura.

### Riscos

| Risco | O que acontece | Mitigação |
|---|---|---|
| Formato | B54, B55, N03 e N07 mudam os três formatos de arquivo | N01 primeiro; fixtures de cada alpha no corpus; `serde(default)` obrigatório |
| Refatoração do estado do app | N05 e B59 unificam três mecanismos e mudam o desfazer; risco de regressão em toda a interface | corpus visual (B53) antes |
| Ponte C++ XDE (N24) | build do vcpkg, IGES não thread-safe | processo filho e mutex |
| Dados de prendedores | NAS, Hi-Lok e lockbolt sem duas fontes abertas | nada [C] entra como confirmado (`confirmado_por ≥ 2`) |
| Escala | ≈ 27 M tokens | repriorizar no fim de cada onda pelo corpus e pelos relatos do usuário |
| Catálogo divergente do código | DES-05 "completo", MON-04 com "limites", STEP "import" marcado feito | reclassificar o catálogo antes da onda 1 para não planejar sobre falsos "ok" |

---

## 7. Pontos não confirmados [C] que precisam de validação

**Comportamento dos CADs de referência** (antes de virar regra de interface):
1. Cores do Edit Part do SW e tooltips do V5 (já citados no ADR 0011).
2. Comprimento automático do Bolted Connection (Inventor).
3. Comportamento exato do Salvar como do SW e do Inventor; Propagate Directory × Send To no CATIA. Hoje só há fontes de fórum.
4. Memorando do Stop Update, Selection Traces e limite da pilha de desfazer no CATIA.
5. Live Section do SW e do CATIA; Shift no Instant3D.
6. Opções de origem e orientação do *positioned sketch* do CATIA além de "Reverse H".

**Normas e dados:**
7. Letras I, O e Q proibidas em datum na ASME; lista de mudanças da ISO 1101:2017.
8. Significado de NAS1291, NAS1919/1921, MS16555, NAS607, HST12, HL19/HL20, colares HL70/HL77/HL86/HL94 e MS21140 como lockbolt. Os dados de nutplate conflitam entre fontes. O ASSIST exige login.
9. Rótulos em `docs/contextos/fixadores.md`: "Rivet Nut = ISO 15977" (a série trata de rebites cegos) e "Drive Rivet = DIN 7337".
10. Licença do crate `dxf`. O NIST STEP File Analyzer pode rodar por linha de comando no CI?

**Código e metas:**
11. Círculo do `Sweep` vira B-spline interpolada em vez de cilindro (`spine_wire`). Provável, não confirmado.
12. Nome dos arquivos de conflito do OneDrive (`-NOMEDOPC`) e detecção de placeholders.
13. Metas de desempenho propostas sem medição:
    - 200 membros estruturais em menos de 2 s;
    - árvore em menos de 2 ms com 300 comandos;
    - 500 pontos de prendedor em 10 s;
    - STEP de 50 MB em 60 s.

**Bugs prováveis ainda não reproduzidos** (reproduzir antes de priorizar):

| Área | Bugs |
|---|---|
| Sketch | filete mantém a cota encurtada; aparar deixa ponto solto; espelhar não iguala raios; `merge_points` |
| Montagem | distância 0 com jacobiano singular e giro de 180°; `auto_kind` e `AlignOnce` |
| Superfícies | `which`, Split e projeção; `PointOnSurface` no plano tangente |
| Desenho | projeção a partir de Topo, Section, Camera e Flat |
| PLM | descarte silencioso ao abrir; `rel_path` só para descendentes; colisão de IDs em pasta compartilhada |
| Prendedores | 1 GDL (o B62 já confirma) |

---

**Entrega**
- **Resultado:** síntese da pesquisa avançada com os 7 capítulos pedidos e a revisão do backlog no JSON abaixo. O JSON tem 37 alterações de itens existentes e 45 itens novos (N01–N45, na ordem da lista `novos`).
- **Evidência:**
  - Lidos o README, as seções 1, 3 e 4 dos 15 relatórios, os handoffs (para os pontos [C]), o mapa macro e os fluxos do `requisitos-macro.md` (§2.0 e §25), o ADR 0011 e o `backlog.md`.
  - Nomes de área conferidos em `scripts/benchmark/backlog.py`.
- **Arquivos alterados:** nenhum. A sessão estava em modo de plano, só leitura, e a ferramenta ExitPlanMode não estava disponível. O documento sai como resposta final, conforme pedido.
- **Verificações:** só leitura. Nada foi compilado nem testado. Contagens de requisitos e de P2/P3 feitas à mão a partir das tabelas dos relatórios (podem ter erro de ±1 por área).
- **Riscos:**
  - Os tokens dos itens novos são estimativas calibradas pelos itens atuais.
  - Os itens alterados que cresceram precisam de nova estimativa: o JSON não tem campo de tokens para alterações, e os valores sugeridos estão na §6.
  - Os prefixos de origem novos (PESQ-SKT…PESQ-FI) convivem com os antigos (PESQ-CAT, PESQ-UX…).

STATUS: DONE_WITH_CONCERNS
SUMMARY: Consolidei os 15 relatórios em 10 princípios, 148 requisitos P0/P1 sem duplicatas com rastreabilidade (todo P0 coberto), um roteiro de ondas que começa pela fundação, 37 alterações e 45 itens novos; as estimativas de tokens e os bugs prováveis ainda precisam ser validados.
