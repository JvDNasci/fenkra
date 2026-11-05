# Benchmark — PMI 3D

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#pmi-3d).

## Como cada ferramenta conduz o fluxo

- **CAD avançado B:** FT&A: planos de anotação, vistas e capturas; tolerâncias e datums ligados às faces.
- **CAD avançado A:** MBD: esquema de cotas automático, vistas de anotação, vistas 3D com câmera e estado.
- **CAD avançado C:** Anotações 3D com Tolerance Advisor que verifica se a peça está totalmente definida.

## O que adotar no Fenkra

- Realce das entidades da anotação e várias chamadas (DR-48).
- Verificação de cotagem completa (consultor de tolerância).

## O que evitar

- Chamadas que atravessam o próprio quadro (DR-47).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| PMI-01 | Cotas e notas 3D | ✅ | Pmi(0..10). |  |
| PMI-02 | GD&T, referências e rugosidade | ✅ | NoteKind Gdt/Datum; símbolo de solda ISO 2553. |  |
| PMI-03 | Vistas e capturas | ✅ | Capturas na árvore. |  |
| PMI-04 | Realce e chamadas inteligentes | 🟡 | Realce falha na montagem; uma seta só; seta atravessa a anotação (DR-31, DR-47, DR-48). | DR-48 |
| PMI-05 | Consultor de tolerância | ❌ |  | BM-PMI-01 |
