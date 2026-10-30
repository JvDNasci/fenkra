# Benchmark — Desenho técnico

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#desenho-técnico).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Folha com fundo (carimbo) separado do trabalho; vistas como snapshot paramétrico do 3D com estado (atualizada/desatualizada, links quebrados); propriedades herdadas norma → folha → vista → elemento.
- **SOLIDWORKS:** Vistas de modelo/projetadas/seção/detalhe/quebra; Model Items importam cotas do 3D; tabelas (BOM, furos, dobras, revisões, lista de corte); AutoBalloon.
- **Inventor:** Base/Projected/Section/Detail/Break Out; Retrieve traz anotações 3D; Parts List e balões automáticos; estilos e normas.

## O que adotar no Fenkra

- Estado de atualização visível por vista (CATIA).
- Cadeia de herança de propriedades norma → folha → vista → elemento (CATIA).
- Vistas por corpo para estrutura soldada (SW).

## O que evitar

- Recarregar silenciosamente sem avisar o que mudou.

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| DES-01 | Folhas, formatos e carimbo | ✅ | Formatos ISO/ANSI, carimbos ISO 7200/ABNT, zonas ISO 5457. |  |
| DES-02 | Vistas base e projetadas | 🟡 | Só 1º diedro; sem 3º diedro (ASME). | BM-DES-01 |
| DES-03 | Cortes e seções | ✅ | Teste vistas_corte_detalhe_cotas_tabelas_e_exportacao. |  |
| DES-04 | Detalhe, auxiliar, quebrada, corte parcial | 🟡 | Detalhe e auxiliar; sem vista interrompida nem corte parcial. | BM-DES-02 |
| DES-05 | Cotas associativas | ✅ | Cotas com estilo, setas e posição do texto. |  |
| DES-06 | Hachuras | 🟡 | Passo fixo 2,5 sem propriedades de padrão/ângulo (layout.rs). | BM-DES-03 |
| DES-07 | Atualização e estado da vista | ❌ | Models::refresh() recarrega por data do arquivo; sem estado visível. | DR-34 |
| DES-08 | Balões e lista de peças | ✅ | Balões automáticos ligados à BOM e à lista de corte por item, inclusive em vistas estruturais (B26). | DR-33 |
| DES-09 | Tabelas | ✅ | TableKind (BOM, CutList, …). |  |
| DES-10 | Propriedades por norma e herança | 🟡 | Estilos por cota/nota; sem cadeia norma→folha→vista completa (DR-35..43). | DR-35 |
| DES-11 | Desenho de estrutura soldada | 🟡 | Lista de corte e balões por item (B26); falta vista própria por corpo/item e detalhe de ponta. | B50 |
| DES-12 | Exportação | ✅ | export.rs; teste de exportação. |  |
