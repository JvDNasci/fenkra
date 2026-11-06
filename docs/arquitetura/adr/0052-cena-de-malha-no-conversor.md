# ADR 0052: cena de malha no conversor, em Rust puro

Estado: aceita. Item B210. Complementa os ADR 0018, 0023 e 0049.

## Contexto
O B210 pede ler e gravar uma dúzia de formatos abertos de malha e visualização. O OpenCASCADE traduz parte
deles (OBJ, glTF, PLY, VRML), mas cada um por uma API diferente, sem 3MF, AMF, X3D, Collada nem USD, e sempre
presos ao documento XDE. O núcleo de malha do B170 ainda não existe: não há corpo de malha na árvore.

## Decisão
1. **Uma cena, N formatos.** `fenkra_conversor::malha::Cena` guarda malhas, materiais e um grafo de nós, sempre
   em milímetros e com Z para cima. Cada leitor chega nela e cada gravador sai dela. A conversão de eixo e de
   unidade fica na borda de cada formato, e nunca entre dois formatos.
2. **Rust puro, sem o kernel.** Os formatos são texto, XML, JSON ou tabelas binárias simples. Ler em Rust dá
   erro em vez de queda num arquivo hostil, e não acrescenta nada à ponte C++. As dependências novas são
   `roxmltree` e `miniz_oxide`, que já estavam na árvore (MIT, Apache-2.0, Zlib). O ZIP é próprio, de 150
   linhas, porque só precisamos de guardado e deflate, com teto de tamanho.
3. **Provedor no registro.** `ProvedorMalha` entra no `Registro`, ao lado do OpenCASCADE. Diálogo, relatório,
   processo filho e matriz de formatos passam a valer para malha sem caminho à parte.
4. **Sólido facetado até o B170.** Sem corpo de malha, a importação sobe a cena para o `ModeloNeutro`: malha
   fechada vira sólido facetado por `Shape::from_mesh`, e aberta vira superfície de referência. Cada triângulo
   vira uma face costurada, então esse caminho tem o teto de 200 mil triângulos do comando de importar malha.
5. **Malha para malha não sobe.** `Registro::converter` entre dois formatos de malha vai cena a cena, sem teto e
   sem perder a cor por vértice.
6. **Partes soltas sobem separadas.** O kernel monta um sólido por casca, e a segunda casca sumiria. Casca do
   lado contrário da maior é cavidade: ela vira sólido à parte, e o relatório avisa, porque subtrair por
   booleana em milhares de faces custaria mais que o resto da importação.
7. **Só arquivo ao lado.** `mtllib` e `uri` só abrem caminho relativo sem subida de pasta: um arquivo hostil não
   pode transformar a leitura numa consulta a arquivos da máquina.

## Fora
O formato fechado que o item pede só para leitura ficou de fora: esta frente só aceita formatos abertos. O leitor citado teria
licença boa, mas a dependência é C e a decisão de levar um formato fechado é de produto. USD e USDZ só gravam, como
o item pede. USD binário (`usdc`) não é gravado: o texto basta para os leitores e o pacote aceita os dois.

## Consequências
- Quando o B170 chegar, a cena é a entrada natural do corpo de malha, e o teto de 200 mil sai.
- Cor por vértice não chega ao documento: o corpo tem uma cor. No caminho malha para malha ela atravessa.
- Quem lê VRML ou X3D de programas que gravam em milímetros sem dizer vê a peça mil vezes maior; o aviso de unidade
  suspeita e a opção de unidade do arquivo resolvem.
