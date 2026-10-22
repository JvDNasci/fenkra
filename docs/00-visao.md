# Documento de Visão

> Rascunho v0.1. Revisar e ajustar.

## 1. Problema

Os CADs paramétricos comerciais (CATIA, SolidWorks, Fusion) são caros, fechados e,
em alguns casos, dependentes de nuvem e assinatura. A principal alternativa aberta, o
FreeCAD, é funcional, mas:

- é **pesado**: a interface trava durante recompute e a visualização é lenta em modelos médios;
- o **desenho técnico 3D → 2D é fraco**: lento, com cotas que se perdem quando o modelo muda
  e pouca automação;
- a parte de **estruturas soldadas** é praticamente inexistente;
- o **BIM** é difícil de usar.

## 2. Proposta

Um CAD 3D paramétrico, nativo para desktop, com:

1. **Desempenho como requisito**: a interface nunca congela e o recompute é incremental e em background.
2. **Desenho técnico como cidadão de primeira classe**: vistas rápidas, cotas associativas,
   BOM e lista de corte automáticas, normas ABNT/ISO.
3. **Estruturas soldadas** completas: membros estruturais, biblioteca de perfis (incluindo
   perfis brasileiros), tratamento de cantos e lista de corte.
4. **Malha → sólido** simples, para aproveitar STLs e escaneamentos de peças prismáticas.
5. Fluxo de trabalho inspirado no melhor de SolidWorks, Fusion e CATIA, **com melhorias**.

## 3. Usuários-alvo

| Perfil | Necessidade principal |
|---|---|
| Projetista mecânico | Peças, montagens e desenhos de fabricação |
| Serralheria / estruturas metálicas | Estruturas soldadas, lista de corte, desenhos |
| Maker / impressão 3D | Modelagem rápida, STL, conversão de malha |
| Arquitetura (fase posterior) | BIM com IFC e pranchas |

## 4. Diferenciais

- Mais leve e responsivo que o FreeCAD.
- Desenho técnico e lista de corte melhores que os de qualquer CAD aberto.
- Gratuito e aberto, sem assinatura nem nuvem obrigatória.
- Normas e perfis brasileiros desde o início.

## 5. Não-objetivos (por enquanto)

- CAM / geração de G-code
- Simulação (FEA, CFD)
- Superfícies classe A e modelagem por subdivisão (estilo T-Splines)
- Leitura de formatos nativos fechados (SLDPRT, CATPart, F3D). A troca é feita via **STEP**.
- Colaboração em tempo real / nuvem
- BIM no MVP (fica para fase posterior)

## 6. Escopo do MVP

Ver [roadmap](roadmap.md). Em resumo: documento e árvore de features, esboço com
restrições, features básicas de sólido, timeline com rollback, import/export STEP/STL,
desenho técnico básico e malha → sólido (níveis 1 e 2).

## 7. Restrições

- Licença do projeto compatível com OpenCASCADE e FreeCAD. Ver [ADR 0003](adr/0003-licenca.md).
- Plataformas: Windows e Linux no MVP; macOS depois.

## 8. Critérios de sucesso do MVP

- Modelar uma peça mecânica típica (suporte, flange, carcaça simples) do esboço ao desenho cotado.
- Alterar uma cota no início da timeline sem quebrar features nem cotas do desenho.
- Abrir um modelo de 500 features sem a interface travar.
- Converter um STL de peça prismática em sólido editável com faces planas e cilíndricas.
