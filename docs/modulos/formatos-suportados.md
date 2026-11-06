# Formatos suportados

> Gerado de `Registro::matriz`; não edite à mão. Cada célula é conferida gravando e lendo um
> modelo de prova em `crates/fenkra-conversor/tests/matriz.rs`. Para regravar: `FENKRA_ATUALIZAR_MATRIZ=1 cargo test -p fenkra-conversor --test matriz`.

| Formato | Extensões | Ler | Gravar | Geometria exata | Malha | Cor | Camadas | Montagem | PMI | Parâmetros | Unidade | Provedor |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| STEP | step, stp | sim | sim | sim | – | sim | sim | sim | – | – | sim | OpenCASCADE |
| IGES | igs, iges | sim | sim | sim | – | sim | – | – | – | – | sim | OpenCASCADE |
| BREP | brep | sim | sim | sim | – | – | – | – | – | – | – | OpenCASCADE |
| DXF | dxf | sim | sim | sim | – | – | sim | – | – | – | sim | Fenkra |
| SVG | svg | sim | sim | sim | – | – | sim | – | – | – | sim | Fenkra |
| IFC | ifc | sim | sim | – | sim | sim | – | – | – | – | sim | Fenkra |
| Árvore paramétrica | fkx | sim | sim | – | – | sim | – | sim | – | sim | sim | Fenkra |
| STL | stl | sim | sim | – | sim | – | – | – | – | – | – | Fenkra |
| OBJ | obj | sim | sim | – | sim | sim | – | – | – | – | – | Fenkra |
| PLY | ply | sim | sim | – | sim | sim | – | – | – | – | – | Fenkra |
| 3MF | 3mf | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| AMF | amf | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| glTF | gltf | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| GLB | glb | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| VRML | wrl, vrml | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| X3D | x3d | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| Collada | dae | sim | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| USD | usda, usd | – | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
| USDZ | usdz | – | sim | – | sim | sim | – | sim | – | – | sim | Fenkra |
