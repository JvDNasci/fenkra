# ADR 0048 — Chapa: fechamento de canto, aba unificada, desdobrar parcial, loft e conversão

## Contexto

Os itens B293, B31 e B35 acrescentam à bancada Chapa metálica o fechamento de canto entre abas, o comando
de aba unificado, a aba entre perfis (lofted), o desdobrar e redobrar parcial e a conversão de um sólido em
chapa. Nenhum deles pede kernel novo em C++: todos se apoiam no que o cálculo das abas já registra.

## Decisão

- **Cada aba de 90° já registra um `SheetMap`** (referencial da aresta, normal, saída, espessura, raio,
  comprimento desenvolvido e reto). O fechamento de canto (`chapa_canto.rs`) lê esses mapas, cruza as retas
  das arestas de duas abas perpendiculares e monta as peças que fecham o vão como prismas de polígonos
  (`Shape::poly_prism`): retângulos para sobreposto e sob, e dois trapézios separados por uma diagonal para o
  topo. A primeira aba é a que chega ao canto andando com o interior da chapa à esquerda, para a propagação
  girar no mesmo sentido em todos os cantos. A planificada usa retângulos da espessura a partir do fim da
  dobra, e toda peça é conferida contra o corpo antes de entrar (volume comum) para não sobrepor material.
  A região do arco da dobra fica aberta (alívio), e abas de outro ângulo que 90° são recusadas em palavras.
- **Aba unificada é um modo da interface, não uma feature nova.** `fenkra-core/src/aba_unif.rs` converte
  entre as cinco features de aba (`SmFlange`, `SmFlangeLoop`, `SmCurveFlange`, `SmProfileFlange`,
  `SmSurfFlange`) levando comprimento, ângulo, raio e direção; os arquivos antigos abrem sem migração. O
  raio próprio nasce de `sheet_efetivo().radius` (regra ou estilo), e o botão de inverter direção é um só
  (`chapa_unif_ui::inverter`). Campo com faixa válida usa `editor::expr_lim` (vermelho com o limite escrito).
- **Desdobrar parcial é um estado por aba, não uma segunda planificação.** `SmUnfoldPartial` põe aba no
  conjunto de desdobradas ou tira dele; o conjunto final vem de todos os comandos da árvore. Quem decide se
  a aba é refeita plana é `Ctx::sheet()`, que consulta o comando em execução num `thread_local`
  (`chapa_desdobra::marca`), porque `Ctx` tem catorze literais de struct espalhados e um campo novo
  obrigaria a mexer em todos. O estado entra na assinatura do cache, para a aba não reaproveitar o resultado
  antigo. Desdobrar todas as abas dá exatamente a planificada global (teste de volume e caixa).
- **Aba entre perfis** faz o loft dos dois contornos e esvazia o sólido tirando as duas tampas (a mesma
  casca da modelagem sólida), com a espessura da chapa. Não há planificação exata: o estado planificado
  avisa e mostra a casca dobrada.
- **Converter em chapa reconhece, não reconstrói.** A espessura é a menor distância entre planos paralelos
  que se enfrentam; as dobras são pares de cilindros coaxiais com raios que diferem da espessura; a prova de
  espessura constante é o volume (metade da área das faces principais vezes a espessura). O corpo não muda.
  O mapa de dobras e a planificação do corpo convertido ficam para uma etapa seguinte.

## Consequências

- O fechamento só enxerga abas feitas pelo cálculo das abas (as que registram `SheetMap`); abas de perfil e
  de superfície não entram.
- Um `thread_local` guarda o comando em execução. É seguro porque cada recompute roda numa thread só, mas
  quem criar um segundo caminho de recompute em paralelo na mesma thread precisa chamar `marca` também.
- A conversão devolve só um aviso com o que reconheceu; a peça convertida ainda não tem planificação própria.
