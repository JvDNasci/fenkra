# ADR 0065: pacote de visualização `.fkv` e visualizador web sem kernel (B181, B338)

## Contexto

O navegador não roda o OpenCASCADE, e o arquivo nativo do Fenkra (`.fkp`, `.fka`, `.fkd`) precisa do kernel
para virar forma. Para ver peça, montagem e desenho na web, no computador e no celular, sem app de loja e
sem modelagem, o Fenkra precisa gerar um arquivo de leitura que já traga tudo calculado. O visualizador
será parte do PLM (B339), mas precisa funcionar separado.

## Decisão

1. **Formato próprio e aberto, `.fkv`**, com especificação em `docs/formatos/pacote-visualizacao.md` e
   versão inteira, como os outros formatos do projeto. É um ZIP com `pacote.json` (manifesto), `cena.glb` e
   `folhas/*.json`. Escolhemos ZIP porque abre em qualquer sistema e o navegador descomprime sozinho
   (`DecompressionStream`), sem biblioteca; o escritor já existia em `fenkra-malha`.
2. **A cena é glTF 2.0 válido**, em milímetros e Z para cima, com raiz de conversão para os leitores
   genéricos. IDs estáveis do documento (`n<id>`, `c<id>`) em `extras`. Quantização de normais e índices de
   16 bits quando cabem, porque o tamanho decide se abre bem no celular. Compressão de malha (meshopt) fica
   para a versão 2: exige decodificador embutido e o ganho já vem do ZIP e da malha compartilhada.
3. **Crate novo `fenkra-pacote`**, acima de `fenkra-solidos`, `fenkra-montagem` e `fenkra-desenho`. O
   desenho sai das mesmas primitivas que a tela e o PDF desenham (`layout::sheet`), como vetor.
4. **Medição exata sem kernel.** Os pontos de captura (pontas de aresta, pontos médios, centros e raios
   de círculos) saem da fórmula de cada aresta do B-rep, em precisão dupla, e vão no manifesto. O
   visualizador prende o toque a eles e calcula em dupla. Sem esses pontos, mede-se na malha e a página
   diz "aproximado".
5. **Decimação protege a propriedade intelectual.** Com a opção ligada, a malha é simplificada
   (`fenkra-malha`) e os pontos exatos não saem, porque entregariam a geometria exata. É a "exportação
   simplificada" do menu e o `--decimar` do `fenkra-cli`.
6. **Uma página só**, `visualizador.html`, com three.js r149 (MIT) embutido, sem rede, que abre o `.fkv`
   de quatro jeitos: arquivo (escolher ou arrastar), embutido na própria página (arquivo autocontido, base64),
   por URL (`?src=`, para hospedar num servidor comum, sem backend) e por mensagem (`postMessage`) quando
   embutida em outra página. Sem pacote embutido ela é o componente que o painel do projeto (B339) usa num
   `iframe`, recebendo o `.fkv` por caminho ou URL. O tema segue as paletas Escuro e Claro do Fenkra.
7. **Gerar o pacote junto ao salvar: avaliado, não ligado.** Custa um recálculo e uma malha a cada
   gravação, o que pesa em montagem grande e engorda o diretório do projeto; o pacote só serve a quem vê, e
   fica velho assim que o documento muda. Recomendação para quando for implementado: opção desligada por
   padrão, no projeto e não no documento, gerando em segundo plano depois de gravar e só quando o hash do
   documento mudou (a geração é determinística, então o hash do pacote serve de chave). O PLM (B339) é o
   lugar natural: gerar na revisão publicada, que é imutável. Por ora o comando Exportar e o `fenkra-cli`
   cobrem o uso.

## Consequências

- O arquivo `.fkv` não volta para o CAD: não tem B-rep nem histórico. Quem precisa editar usa o nativo.
- Tamanho medido (corpus do teste, placa de 14 mil triângulos): 100 instâncias da mesma peça pesam 96 KB;
  100 peças todas diferentes pesam cerca de 9 MB, ou 3,2 MB com 30% dos triângulos, dentro da meta de 10 MB.
  A página autocontida soma cerca de 700 KB do three.js.
- Navegadores sem `DecompressionStream` (anteriores a Chromium 80, Firefox 113 e Safari 16.4) mostram o
  motivo em vez de abrir. Um descompressor próprio é uma opção se o público pedir.
- Não há GPU obrigatória além do WebGL 1, que todo navegador de celular tem.
- O visualizador não mede malha decimada com a exatidão do CAD, e o aviso aparece na medida.
