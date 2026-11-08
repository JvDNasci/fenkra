# ADR 0082: IFC da estrutura de aço escrito em Rust, sem biblioteca externa (B134)

## Contexto

O B134 pedia `IfcMember` e `IfcPlate` por meio de uma biblioteca dinâmica de IFC de código aberto. Ela é
grande, em C++, traz o próprio núcleo geométrico ligado a outra versão do OpenCASCADE que a 8.0.1 do Fenkra, e
resolve sobretudo a leitura e a conversão de sólidos paramétricos. Para gravar a estrutura não precisamos dela:
o Fenkra já tem o sólido cortado de cada membro e o referencial local dele (`structural_members`), e o conversor
já escreve IFC4 facetado em Rust (ADR 0055).

## Decisão

1. `fenkra_solidos::st_ifc::ifc_da_estrutura` escreve IFC4 direto em texto STEP físico. Membro vira
   `IfcMember` (`.MEMBER.`), placa e reforço viram `IfcPlate`, cordão sólido vira `IfcFastener` (`.WELD.`).
2. O corpo é a malha do sólido já cortado (`IfcTriangulatedFaceSet`), uma vez por item da lista de corte num
   `IfcRepresentationMap`; cada peça entra pelo referencial local do membro (`IfcLocalPlacement`). Assim
   meia-esquadria, encaixe e furos chegam iguais ao modelo.
3. O perfil vai como `IfcMaterialProfileSet` com a seção paramétrica do esquema (I, U, L, T, tubos, barras), o
   material como `IfcMaterial`, e comprimento de corte e volume como quantidades de base (`Qto_*BaseQuantities`).
4. Unidade de comprimento em milímetro; volume em metro cúbico.

## Consequências

- Nenhuma dependência nova nem conflito de versão do kernel; o arquivo volta pelo leitor de IFC do conversor
  com o mesmo volume e a mesma caixa (teste `fenkra-conversor/tests/ifc_estrutura.rs`).
- O corpo é facetado, não um sólido por extrusão: quem abre vê a forma exata do corte, mas não recebe a peça
  como extrusão editável. Gravar `IfcExtrudedAreaSolid` com recortes por `IfcBooleanClippingResult` fica para
  quando houver pedido de troca com análise estrutural.
- A leitura de IFC paramétrico continua como no ADR 0055: é lá que uma biblioteca completa faria diferença.
