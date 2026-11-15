# Ida e volta por formato (B211)

> Gerado de `crates/fenkra-conversor/tests/ida_e_volta.rs`; não edite à mão. Cada linha grava um modelo
> montado em código, lê de volta e compara. No 2D: curvas, textos e extensão em milímetros. No 3D: sólidos,
> volume e caixa. Para regravar: `FENKRA_ATUALIZAR_IDA_E_VOLTA=1 cargo test -p fenkra-conversor --test ida_e_volta`.
> O DWG não entra na conta porque depende do LibreDWG instalado à parte; o encanamento é conferido em
> `tests/dwg.rs` com um programa falso, e com o de verdade quando ele existe. O 3DM pede o openNURBS baixado por
> `scripts/baixar_opennurbs.py`; o volume dele é o da integração adaptativa (`Shape::volume_preciso`).

| Formato | Caminho | Ida | Volta | Resultado |
|---|---|---|---|---|
| DXF | esboço da peça | 6 curvas, 80,000 × 50,000 | 6 curvas, 80,000 × 50,000 | igual |
| SVG | esboço da peça | 6 curvas, 80,000 × 50,000 | 6 curvas, 80,000 × 50,000 | igual |
| DXF | folha do desenho | 5 curvas e 1 texto, 125,500 × 87,500 | 5 curvas e 1 texto, 125,500 × 87,500 | igual |
| SVG | folha do desenho | 5 curvas e 1 texto, 125,500 × 87,500 | 5 curvas e 1 texto, 125,500 × 87,500 | igual a 0,01 mm |
| IFC | peças (gravar e ler) | 2 sólidos fechados, 540000000 mm³, 4300,000 × 300,000 × 3000,000 | 2 sólidos fechados, 540000000 mm³, 4300,000 × 300,000 × 3000,000 | igual |
| IFC | extrusão de perfil de aço (ler) | 2 sólidos fechados, 6505000 mm³, 450,000 × 190,000 × 1000,000 | 2 sólidos fechados, 6505000 mm³, 450,000 × 190,000 × 1000,000 | igual |
| DWG | esboço e folha | DXF R2000 pelo dxf2dwg | DXF pelo dwg2dxf | com o LibreDWG instalado |
| 3DM | peça com camadas (gravar e ler) | 1 sólidos fechados e 1 abertos, 30869 mm³, 140,000 × 40,000 × 16,296; camadas Chapas > Corte e Superfícies; 2 cores | 1 sólidos fechados e 1 abertos, 30869 mm³, 140,000 × 40,000 × 16,296; camadas Chapas > Corte e Superfícies; 2 cores | igual |
| PDF 3D | peça | – | sem biblioteca aberta de PRC com licença compatível (ADR 0055) | fora |
| STEP em XML | peça | – | o OpenCASCADE 8.0.1 não traz a parte 28 (ADR 0055) | fora |
