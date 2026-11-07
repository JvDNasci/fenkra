# Tendências de IA em CAD paramétrico e o que o kart ensinou

Data: novembro de 2026. Os números entre colchetes remetem às fontes no fim. Conferi [1], [2], [3] e [9] no
texto do artigo; as demais fontes vieram de resumo ou de página de projeto, então trate-as como indício. Não
abri as imagens do diário: cito os arquivos pelo relatório.

## Conclusão

O campo se move de "gerar a peça a partir de um texto" para "gerar, medir e corrigir em ciclo fechado". Os
ganhos medidos vêm do ciclo, não do modelo. É aí que o Fenkra pode ganhar: não precisa ter IA própria, precisa
ser o melhor lugar para um agente externo trabalhar e provar que acertou. Hoje ele tem uma boa porta
(`fenkra-mcp`), mas estreita, e sem olhos. O kart não passou por ela.

## O que está maduro e o que é só promessa

| Tema | Estado | Evidência |
|---|---|---|
| Código de modelagem gerado por modelo de linguagem, peça simples | Funciona, com conferência | No nível básico de [1], a invalidez fica perto de 15%, mas o melhor IoU é só 0,62 |
| O mesmo, peça complexa | Promessa | Em [1], a invalidez sobe a 70–90% na maioria dos modelos no nível difícil, puxada por varredura, loft e casca; IoU máximo de 0,28. Os autores dizem que o problema segue em grande parte sem solução |
| Ciclo com medida do kernel e juiz visual | Maduro como método; ainda não como produto | [2]: IoU mediano de 0,81 para 0,96 e Chamfer médio de 28,37 para 0,74. [4] e [5] seguem a mesma linha. Ressalva: benchmark dos próprios autores, 100 pedidos, só peça única |
| Testes executáveis como critério de acerto | Tendência forte | [3]: 5.937 testes; concordância com pessoas de 0,938, contra 0,474 da nota visual por modelo. AUC 0,928, contra 0,663 do Chamfer |
| Servidores MCP para CAD | Real e novo, alcance irregular | Um grande fornecedor lançou servidores oficiais em abril de 2026 [6][7]. O servidor do FreeCAD roda scripts e simulação [8]. A oficialidade de outro servidor é contraditória entre fontes [6][23]. "MCP" no rótulo não diz quanto da API está exposta [6] |
| Autonomia total de agentes em engenharia | Promessa | 33 entrevistas em 28 empresas [9]: o que existe é autonomia limitada, com revisão humana. Estudo qualitativo, amostra não aleatória |
| Conferência por regras (desenhos, requisitos, dados) | Maduro, e pouco glamouroso | [9] coloca aqui os ganhos de curto prazo |
| Copilotos com ganho de produtividade | Sem evidência | Não achei estudo controlado. O número de "66%" que circula não tem origem identificável; descartei |
| Reconhecimento de features em B-Rep | Maduro em dado sintético, frágil no real | 99,53% em conjunto de teste padrão [10], mas [11] registra queda perceptível em peças reais e usa transferência para compensar; [12] ataca a falta de rótulos |
| Malha por texto ou imagem | Serve para conceito e visual | Saídas sem estanqueidade e com arestas lavadas [13]; viram entrada de uma etapa de conversão [14] |
| Imagem ou varredura para B-Rep editável | Pesquisa | [15] e [16] são avanços, com pouca evidência em peça real. [17]: qualidade cai com a complexidade e o ranking muda conforme a métrica |
| Desenho e cotagem automáticos | Promessa | Regras clássicas põem cotas demais [18]. Ler desenho já vai melhor, com F1 de 86,29% em só 20 pares e revisão humana [19]. Não achei benchmark de geração, nem confirmei em fonte forte a geração de PMI |
| Simulação por rede substituta | Útil dentro do envelope treinado | O limite é sair da distribuição [20][21][22]. Boa parte dos números vem de pré-publicação ou de fornecedor |

Atenção a um contraste: [2] parte de 95% de execução, e [1] mostra 70–90% de invalidez no nível difícil. Não
há contradição, só benchmarks diferentes. Nenhum número isolado representa o campo, e [1] insiste que
executar, acertar a forma e entender a intenção são capacidades quase independentes.

## Onde o Fenkra está bem

- **Contenção:** lista fechada de comandos, pasta confinada, STEP só com aceite, nada de rede (ADR 0039). É o
  que a segurança industrial pede, com ambientes locais ou isolados [9]. O servidor do FreeCAD ganha em alcance
  por rodar script livre [8], mas perde nesse controle.
- **Medida no ciclo:** `comparar_alvo` diz o que divergiu e onde, com status e graus de liberdade por item. É
  metade do que [2] mostra funcionar.
- **Rastreabilidade:** `registro` e `reproduzir` atendem ao pedido de log e diferenças de [9].
- **Resultado editável:** sai árvore com parâmetros, não só um script ou um corpo importado.
- **Sistema inteiro:** os benchmarks param em peça única ([1] e [2] dizem isso). O kart foi além, com esqueleto,
  montagem, desenho, PMI e PLM. Não é comparação justa (ver limites abaixo), mas o território é pouco explorado.

## Onde está atrás

- **Alcance:** o MCP tem 18 ferramentas. Para modelar: plano, esboço, extrusão, furo, filete e chanfro. Não há
  revolução, padrão, varredura, chapa, estrutural, montagem, desenho nem PMI. `features` existe no CLI, mas não
  no MCP. O próprio guia admite que montagem grande e peça orgânica não são o caso bom.
- **Sem olhos:** nenhuma ferramenta devolve imagem. `visualizar` gera `.fkv`, e a foto depende do aplicativo
  aberto com `FENKRA_DEMO_FOTO`.
- **Sem critério além do alvo:** `comparar_alvo` exige um STEP pronto. Em projeto novo não há alvo, e não existe
  forma de declarar requisitos.
- **Sólido inválido:** não achei conferência geral de validade no recálculo, na `Sessao` nem no `confere` do kart.
  `is_valid` só é chamado em dois pontos específicos. O K-10 passou por essa brecha.
- **Sem simulação e sem cotagem de folha:** a cotagem automática encontrada é a do esboço.

## Oportunidades, em ordem

| # | O quê | Por quê | Valor | Esforço |
|---|---|---|---|---|
| 1 | Validade do corpo no status do recálculo, na API e no app | Fecha a pior falha silenciosa. Rodar antes em RD-02: o K-10 é hipótese, pode exigir checar orientação também | Alto | Baixo |
| 2 | Ferramenta `ver`: imagem sem aplicativo aberto, várias vistas e corte | [2]: tirar a visão faz o Chamfer médio ir de 0,74 a 18,19. Três vistas fixas deixaram passar folgas, então precisa de corte e zoom | Alto | Médio |
| 3 | `verificar`: requisitos declarados na peça, com resultado por requisito | Tendência de [3], útil também para quem não usa IA. Massa, folga, interferência, furos, e "mudei o parâmetro, o conjunto seguiu?" | Alto | Médio |
| 4 | Erros acionáveis: item, campo, causa e sugestão, além de `codigo` | K-02, K-08, K-18, K-13. Hoje o agente adivinha pelo texto | Alto | Baixo a médio |
| 5 | Ampliar a API por ondas, a partir do esquema dos comandos | Primeiro revolução, padrão e espelho; depois montagem e links; depois chapa e estrutural. Lista escrita à mão não escala | Alto | Alto |
| 6 | Seleção semântica: "aresta mais próxima", "gerada por", filtro por tipo | K-13 custou tentativas. Também reduz a fragilidade de aresta por ponto na chapa | Médio a alto | Médio |
| 7 | Descoberta: `comandos` com esquema e consulta à biblioteca | O agente fez à mão o que o gerador faz em 1 chamada. Expor `features` no MCP entra aqui | Médio | Baixo |
| 8 | Ampliar o reconhecimento (bolsos, rasgos, padrões) e sugerir cotas por regra, com revisão | Hoje são 4 tipos. Base para desenho assistido; regras antes de rede neural, por [18] e [19] | Médio | Médio a alto |

**O que não fazer agora:** embutir modelo próprio, treinar gerador de peças, criar texto-para-sólido na
interface, imagem para CAD ou simulação por rede substituta. São caros, mudam rápido e não diferenciam.

## O que o kart ensinou

**Limites deste dado.** Foi uma rodada só, e o relatório não mede tokens nem tempo do agente. Ele escreveu Rust direto nas
crates, com acesso ao código-fonte (2.544 linhas, 70 peças), sem usar MCP, CLI nem Python: o `Cargo.toml` do
kart não depende deles. Com o MCP atual o kart não sairia, porque falta até revolução. Prova a capacidade do
núcleo, não a da porta para agentes.

**API**
- Quase tudo foi preenchido em campos do documento, com atalhos próprios em `apoio.rs` (`sk_rect`, `tubo`,
  `dobra_nos`). Esses atalhos são um rascunho da API de alto nível que falta.
- O esqueleto ponto a ponto por expressão pede sketch 3D. Sem ligar posição de montagem ao mestre (F-07), o agente
  gravou números, e a regressão não chega à dianteira.
- Convenções implícitas custam tentativas: sufixo em `mestre_EE1` (K-01), sentido do corte oposto ao da adição
  (K-14), ponto médio da aresta circular do lado oposto da costura (K-13), `y` da tabela contado de baixo (K-19).
- Sem descoberta, o agente vai pelo caminho conhecido. Coroa e corrente saíram à mão, em cerca de 30 linhas e três
  peças, antes de medir o gerador. Modelou em lote, e só depois pensou a fabricação.
- A forma livre usou `novo_para_teste`, que a interface não tem: um resultado que o usuário não reproduz.

**Erros**
- Mensagem que aponta a causa errada é pior que erro nenhum: "linha de comprimento zero" (K-02), erro de superfície
  aparecendo no Engrossar seguinte (K-08), "sem tamanho compatível" para Ø8,5 (K-18).
- Aviso em situação normal treina a ignorar aviso (K-21), tanto gente quanto agente.
- Resultado zero precisa ser falha: "0 peças atualizadas" (K-04) e vista vazia (K-03) passaram como sucesso.
- Falha total sem saída parcial trava o lote: uma peça derruba o pacote (K-05, K-06).

**Verificação.** O `confere` só lia status e a caixa batia. Por isso "ok" significa "calculou". Cada falha silenciosa
pede uma camada diferente, e isso define a ordem das oportunidades:

| Camada que pegaria | Casos |
|---|---|
| Kernel e faixa plausível | K-10; massa de 1415,9 kg com bico e pontão maciços; material desconhecido aceito (F-06) |
| Regra ou teste declarado | K-04, K-03, K-09 (ponta com corte reto), K-19, K-27 |
| Imagem | K-25 (tudo cinza), K-22, K-24, K-26, e os bicos no quadro |

A visão é necessária, mas não basta: as capturas só vieram no fim, em lote, 7 segundos ou mais cada, e a captura
padrão pegou a tela de abertura em PO-01. Ver a peça durante a modelagem, e não depois, é a mudança real. O relatório
de interferência mostra o outro lado: 111 achados, em boa parte ruído. Verificação sem triagem também cansa.

## Pontos para o roteiro do vídeo

1. **Gancho:** 70 peças, 2.544 linhas, um agente de IA, um CAD open source em alpha. Dizer logo que foi por código,
   não pela interface, e que nada passou por mouse real.
2. **Contraste:** benchmarks de 2026 medem uma peça por vez, e mesmo assim o texto-para-CAD falha muito na complexa.
   O kart seguiu outro caminho: código, recálculo e correção em ciclo. Não é prova de superioridade, é outro método.
3. **A virada:** "ok" não quer dizer certo. O aro calculou, a caixa bateu, e a interferência acusou volume falso (K-10).
   E o kart pesando 1415,9 kg, número que qualquer pessoa estranharia.
4. **Antes e depois:** à mão, cerca de 30 linhas; no gerador, 1 chamada com 5 peças, 2 juntas e 104 elos.
5. **Mostrar as falhas:** `0054_ex-12_falhou.png` e `0055_ex-12_segunda_tentativa.png`; `0091_kart_geral.png` (cinza)
   contra `0132_kart_geral_colorido.png`, que conta o K-25.
6. **Tese:** a IA não substituiu o engenheiro; mostrou o que o programa precisa dizer sobre si mesmo. Evitar "a IA
   projetou um kart": motor e banco são volumes simples, e a montagem tem posições gravadas.
7. **Fecho:** o próximo passo não é IA dentro do Fenkra, é o Fenkra enxergar, testar e explicar erros para qualquer agente.

## Fontes

1. Benchmark Text2CAD-Bench: https://arxiv.org/html/2605.18430v1
2. CADSmith, geração em múltiplos agentes com validação geométrica: https://arxiv.org/html/2603.26512
3. Avaliação de texto para CAD com testes executáveis: https://arxiv.org/html/2605.07807v1
4. CADCodeVerify, correção por retorno visual: https://arxiv.org/abs/2410.05340v1
5. Agente de projeto e otimização com ciclo de renderização: https://arxiv.org/pdf/2511.17511
6. Comparação de servidores MCP para CAD (blog de empresa do setor, usar com cautela): https://www.getleo.ai/blog/cad-mcp-server-comparison-2026
7. Anúncio de servidores MCP oficiais de um fornecedor: https://www.pulse.bot/ai/news/autodesk-announces-fusion-mcp-servers-and-more-ai-updates-7c41cbec-4130-4345-a31e-929e65cfacec/
8. Servidor MCP livre para o FreeCAD: https://github.com/neka-nat/freecad-mcp
9. Entrevistas sobre IA agêntica em engenharia e manufatura: https://arxiv.org/html/2604.09633
10. Reconhecimento de features por rede em grafo, 2025: https://zrb.bjb.scut.edu.cn/CN/abstract/abstract14407.shtml
11. BrepMFR, reconhecimento em B-Rep com transferência: https://buaa.globalimpact.cn/en/publications/brepmfr-enhancing-machining-feature-recognition-in-b-rep-models-t/
12. BRepMAE, pré-treino com poucos rótulos: https://arxiv.org/pdf/2602.22701
13. CADFS, conjunto e método para programas CAD: https://arxiv.org/pdf/2605.01925
14. CADFit, de malha para programa CAD: https://arxiv.org/pdf/2605.01171
15. Img2CADSeq, de imagem para sequência CAD: https://arxiv.org/abs/2605.13293
16. BrepGaussian, B-Rep a partir de várias vistas: https://arxiv.org/html/2602.21105v1
17. CADBench, avaliação de reconstrução: https://chatpaper.com/paper/278689
18. Patente sobre cotagem automática de desenhos: https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/11954820
19. Ligação de anotações 2D a features 3D com apoio de modelo de linguagem: https://www.emergentmind.com/papers/2602.18296
20. GLOBE, generalização fora da distribuição em CAE: https://arxiv.org/pdf/2511.15856
21. Transferência de geometria entre famílias de veículos: https://arxiv.org/abs/2605.27968
22. Rede substituta em tensão de suporte, NAFEMS: https://www.nafems.org/publications/resource_center/nwc25-0007109-paper/
23. Resenha de servidores MCP (sem teste prático): https://chatforest.com/reviews/interior-design-architecture-mcp-servers/
