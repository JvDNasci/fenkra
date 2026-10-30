# Benchmark

Comparação do Fenkra com CATIA V5, SOLIDWORKS e Inventor usando **só material público**.

- [Catálogo de features](catalogo.md): 154 features, nome em cada ferramenta, como funcionam, status no Fenkra.
- Dados em [catalogo.json](catalogo.json) (alimenta o comparador interativo).
- [Fontes](fontes.md) e [metodologia](#metodologia).

## Workflow por módulo

- [Sketch](sketch.md)
- [Sólidos](solidos.md)
- [Superfícies e wireframe](superficies-e-wireframe.md)
- [Chapa metálica](chapa-metalica.md)
- [Estrutural e soldas](estrutural-e-soldas.md)
- [Montagem](montagem.md)
- [Desenho técnico](desenho-tecnico.md)
- [PMI 3D](pmi-3d.md)
- [Navegação](navegacao.md)
- [Biblioteca de normas](biblioteca-de-normas.md)
- [PDM / PLM](pdm-plm.md)
- [Geral](geral.md)

## Metodologia

1. Lista mestra montada pela união das barras de comandos públicas das três ferramentas, deduplicada com nome canônico em português.
2. Cada entrada descreve o funcionamento comum na ordem **suporte → modo → parâmetros → avançado → saída** (filosofia do design review, DR-25..29).
3. Status no Fenkra lido no código (`FeatureKind`, `RefKind`, ferramentas do sketch, comandos da barra) e nos testes, na versão 0.1.0-alpha.26.
4. Lacunas viram IDs de backlog: `DR-NN` (design review), `CTX-*` (contextos) ou `BM-<módulo>-NN` (achadas aqui).
5. **Limitação:** a documentação oficial do CATIA V5 exige login; usamos apostilas universitárias abertas, documentação de API pública e tutoriais. As fontes de PDM/PLM são fracas (comparativos).
