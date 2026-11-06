# ADR 0053: o que entra e o que fica de fora nos formatos CAD e 2D

Estado: aceita. Item B211, entregue em parte. Complementa os ADR 0049 e 0052.

## Contexto
O B211 lista 3DM, IFC, PDF 3D, DWG, SVG, DXF de esboço e STEP em XML. A regra desta frente é só formato aberto, com
biblioteca de licença compatível, sem dependência pesada.

## Decisão
1. **SVG e DXF pelo mesmo desenho.** `plano.rs` traduz os esboços para o `Desenho` do `fenkra-dxf`, e o SVG lê e grava
   esse mesmo tipo. A volta ao documento é a do DXF, em `dxf::importar_desenho`. Círculo e arco atravessam exatos.
2. **Y do SVG.** O arquivo do Fenkra grava `data-fenkra-y0`, para as coordenadas voltarem iguais. Num SVG de fora, o Y
   zero é a borda de baixo da folha.
3. **IFC facetado, em Rust.** IFC é STEP físico, mas o leitor do kernel só conhece os esquemas de STEP, e a biblioteca
   que resolve IFC por inteiro é pesada. A leitura traz faces trianguladas, poligonais, B-rep facetado e instâncias
   por `IfcMappedItem`, e entra na cena de malha do ADR 0052. A gravação usa `IfcBuildingElementProxy` com a malha uma
   vez só em `IfcRepresentationMap`.

## Fora, e por quê
- **Sólido paramétrico de IFC** (extrusão e varredura): é a forma mais comum em arquivos reais, e o relatório conta
  quantos ficaram de fora. Resolver isso pede um núcleo de IFC ou a biblioteca citada no B134.
- **3DM:** a biblioteca aberta é C++ grande, e o formato não se escreve à mão com NURBS exatas. Pede uma ponte própria.
- **PDF 3D (PRC):** não há biblioteca aberta compatível, e o codificador de PRC é trabalho de uma frente inteira.
- **DWG:** a biblioteca aberta é GPL, e a outra tem licença comercial. O `deny.toml` recusa a primeira como dependência;
  como processo externo ela exigiria decisão de produto e instalação à parte.
- **STEP em XML:** o OpenCASCADE 8.0.1 não traz leitor nem gravador da parte 28 (os cabeçalhos foram conferidos).

## Consequências
Cada célula da matriz de formatos segue conferida por teste. Formato novo entra pelo provedor ou por `plano.rs`.
