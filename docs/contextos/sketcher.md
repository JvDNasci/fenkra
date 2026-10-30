<!-- origem: contexto sketcher.txt (contexto do usuário, gerado com assistente de IA) -->

```text

Manual Técnico Profissional Completo

Inputs • Outputs • Parâmetros • Modos de Operação • Tolerâncias • Regras de Continuidade • Casos Especiais • Boas Práticas • Interação com Módulos

1. INTRODUÇÃO AO SKETCHER

O Sketcher é o modelador 2D paramétrico d Ele funciona como um sistema de equações geométricas, onde cada geometria adiciona variáveis e cada constraint adiciona relações matemáticas. Ele é a base para a criação de sólidos, superfícies, perfis de chapa, perfis de usinagem e guias de modelagem.

2. GEOMETRIAS DO SKETCHER

2.1 POINT (Ponto)

Inputs

Coordenadas (X,Y)

Interseção

Projeção

Ponto médio

Ponto proporcional

Parâmetros

Tipo de ponto

Sistema de referência

Outputs

Ponto paramétrico

Tolerâncias

Coincidência: 0.001 mm

Casos Especiais

Pontos projetados podem perder associatividade.

Boas Práticas

Prefira pontos paramétricos.

2.2 LINE (Linha)

Inputs

Dois pontos

Ponto + direção

Horizontal/Vertical

Parâmetros

Comprimento

Direção

Limites

Outputs

Segmento de linha

Tolerâncias

Precisão angular: 0.01°

Casos Especiais

Linhas horizontais/verticais podem quebrar com Draft.

Boas Práticas

Use constraints de direção.

2.3 CIRCLE / ARC (Círculo / Arco)

Inputs

Centro + raio

Três pontos

Dois pontos + raio

Parâmetros

Raio

Ângulos

Outputs

Arco ou círculo

Tolerâncias

Fechamento: 0.001 mm

Casos Especiais

Arcos > 360° são rejeitados.

Boas Práticas

Fixe o centro.

2.4 SPLINE

Inputs

Pontos

Tangentes

Curvaturas

Parâmetros

Grau

Tensão

Suavidade

Outputs

Curva spline

Casos Especiais

Pontos muito próximos geram oscilação.

Boas Práticas

Use poucos pontos.

2.5 PERFIS (Rectangle, Slot, Profile, etc.)

Inputs

Pontos

Direções

Parâmetros

Largura

Altura

Ângulo

Outputs

Perfis fechados

Casos Especiais

Perfis automáticos criam constraints excessivos.

Boas Práticas

Prefira perfis manuais.

3. CONSTRAINTS

3.1 DIMENSIONAL

Length

Distance

Angle

Radius

Diameter

Tolerâncias

Linear: 0.001 mm

Angular: 0.01°

Casos Especiais

Dimensões redundantes geram over‑constraint.

Boas Práticas

Evite duplicação.

3.2 GEOMETRICAL

Coincidence

Concentricity

Parallelism

Perpendicularity

Tangency

Symmetry

Horizontal/Vertical

Midpoint

Casos Especiais

Constraints contraditórios travam o solver.

Boas Práticas

Use Fix apenas quando necessário.

4. FERRAMENTAS DE EDIÇÃO

4.1 TRIM / QUICK TRIM

Inputs

Curvas

Casos Especiais

Pode quebrar constraints.

Boas Práticas

Revise constraints após Trim.

4.2 EXTEND / SHORTEN

Inputs

Linha ou arco

Casos Especiais

Pode inverter direção.

Boas Práticas

Fixe o ponto inicial.

4.3 MIRROR

Inputs

Geometria

Linha de espelho

Casos Especiais

Constraints duplicados geram conflitos.

Boas Práticas

Desative “Copy Constraints” em perfis complexos.

4.4 OFFSET

Inputs

Curva ou perfil

Casos Especiais

Splines podem gerar auto‑interseção.

Boas Práticas

Use offset manual para splines.

4.5 DRAFT (Inclinação)

Inputs

Geometria

Pivot

Referência

Parâmetros

Ângulo

Direção

Propagação

Casos Especiais

Perfis fechados podem abrir.

Boas Práticas

Use Draft apenas em perfis limpos.

5. SOLVER PARAMÉTRICO

Modos

Under‑constrained

Fully‑constrained

Over‑constrained

Tolerâncias

Coincidência: 0.001 mm

Angular: 0.01°

Fechamento: 0.001 mm

Casos Especiais

Sketches importados têm tolerâncias ruins.

Boas Práticas

Busque sempre Fully Constrained.

6. INTERAÇÃO COM OUTROS MÓDULOS

6.1 PART DESIGN

Usos

Pad

Pocket

Shaft

Groove

Rib

Slot

Hole

Regras

Sketch fechado para sólidos

Sketch limpo para estabilidade

Casos Especiais

Auto‑interseção → falha

Boas Práticas

Use Sketch Analysis.

6.2 GSD (Superfícies)

Usos

Extrude

Sweep

Fill

Blend

Multi‑Section

Regras

Sketch suave

Evitar cantos agudos

Casos Especiais

Splines com muitos pontos → torção

Boas Práticas

Use splines de grau 5.

6.3 DRAFTING

Usos

Projeção

Cotas

Geometria 2D

Casos Especiais

Splines → cotas instáveis

Boas Práticas

Mantenha sketches organizados.

6.4 ASSEMBLY DESIGN

Usos

Referências

Planos

Casos Especiais

Fix excessivo → peça rígida

Boas Práticas

Use sketches apenas como referência.

6.5 SHEET METAL

Usos

Flanges

Walls

Cutouts

Regras

Linhas + arcos

Sem splines

Casos Especiais

Ângulos agudos → falha

Boas Práticas

Mantenha ângulos > 30°.

6.6 FREESTYLE / IMAGINE & SHAPE

Usos

Guias

Limites

Casos Especiais

Constraints rígidos travam deformação

Boas Práticas

Splines limpas.

6.7 MACHINING

Usos

Contornos

Guias

Casos Especiais

Splines → polilinhas no CAM

Boas Práticas

Use linhas e arcos.

6.8 ANALYSIS (FEA)

Usos

Perfis estruturais

Limites de malha

Casos Especiais

Splines → malha ruim

Boas Práticas

Perfis simples.

7. RESUMO PROFISSIONAL

O Sketcher é o núcleo paramétrico do. Ele alimenta todos os módulos e exige geometria limpa, constraints bem definidos e perfis adequados ao tipo de modelagem.

FIM DO DOCUMENTO

Este arquivo contém o manual técnico completo do Sketcher e sua interação com todos os módulos do CATIA V5.
```
