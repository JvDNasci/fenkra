# ADR 0086: Leitura de FBX em Rust e extrusão do IFC como malha (B210, B211)

- **Status:** aceita
- **Data:** 2026-11-13
- **Revê:** [ADR 0052](0052-cena-de-malha-no-conversor.md) (o FBX entra, só leitura) e
  [ADR 0055](0055-formatos-cad-e-2d-abertos.md) (o sólido por extrusão do IFC deixa de ficar de fora)

## Contexto
O ADR 0052 deixou o FBX de fora porque o leitor citado no item é uma dependência em C. O B210 pede a leitura;
arquivos FBX chegam de visualização, de levantamento e de outros programas, e quem recebe precisa ao menos
ver e medir. No IFC, o ADR 0055 só lia malha pronta: a forma mais comum nos arquivos de edificação, o perfil
extrudado (`IfcExtrudedAreaSolid`), ficava de fora com aviso.

## Decisão
- **FBX lido em Rust puro**, sem dependência nova: a árvore de nós binária (com vetores comprimidos em zlib,
  abertos pelo `miniz_oxide` que já está no conversor) e a forma texto viram a mesma árvore, e dela a cena
  do ADR 0052. Entram malhas, materiais com cor, modelos com posição, giro (XYZ) e escala, instâncias (o
  mesmo modelo sob dois pais), unidade e eixo de cima. Pivôs, animação, esqueleto, câmera, luz e textura vão
  para o relatório.
- **Só leitura.** O Fenkra não oferece gravar FBX: o formato é fechado e esta frente só grava formatos
  abertos. O gravador mínimo do módulo existe para os testes e para a sonda da matriz conferir a leitura.
- **Extrusão do IFC vira malha**: perfil retângulo, círculo (64 lados) ou contorno livre de retas, triangulado
  por orelhas (aceita perfil côncavo), com tampas e laterais, no lugar do sólido. Perfil com furo entra pelo
  contorno de fora e o relatório conta o furo. Revolução, varredura e booleana do IFC seguem de fora.

## Consequências
- A matriz de formatos ganha a linha FBX (ler sim, gravar não), conferida pela sonda com o arquivo de prova.
- 3DM, PDF 3D e DWG continuam esperando decisão de produto (ADR 0055). STEP em XML segue sem suporte no
  OpenCASCADE 8.0.1.
- Um FBX com pivô ou pré-rotação chega com a posição sem essas correções; o relatório avisa quantos modelos
  tinham.
