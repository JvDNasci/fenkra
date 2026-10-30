<!-- origem: Contexto superfícies.txt (contexto do usuário, gerado com assistente de IA) -->

```text
Manual Técnico Profissional – Detalhamento Completo

Inputs • Outputs • Parâmetros • Modos de Operação • Tolerâncias • Regras de Continuidade • Casos Especiais • Boas Práticas

1. WIREFRAME

1.1 POINT (Ponto)

Inputs

Coordenadas (X,Y,Z)

Curva (parâmetro U ou comprimento)

Superfície (parâmetros U,V)

Ponto + direção

Interseção de elementos

Ponto médio ou proporcional

Parâmetros

Tipo de ponto (Coordinates, On Curve, On Surface, Between, Tangent, Normal)

Método de parametrização (comprimento real ou normalizado)

Direção (vetor, curva, superfície)

Outputs

Ponto cartesiano ou paramétrico

Modos de operação

Paramétrico (associativo)

Não paramétrico (fixo)

Tolerâncias

Tolerância de projeção: 0.001 mm

Tolerância UV de superfície: depende da qualidade da superfície

Regras de continuidade

Não se aplica (pontos são entidades C0)

Casos especiais

Pontos em superfícies degeneradas podem falhar

Pontos em curvas fechadas podem gerar ambiguidade de parametrização

Boas práticas

Prefira pontos paramétricos para modelos atualizáveis

Evite pontos por coordenadas em superfícies complexas

1.2 LINE (Linha)

Inputs

Dois pontos

Ponto + direção

Interseção de planos

Tangente de curva

Normal de superfície

Parâmetros

Comprimento (simétrico ou unilateral)

Direção (vetor, curva, superfície)

Limites (ponto inicial/final)

Outputs

Linha limitada ou infinita

Modos de operação

Direção fixa

Direção associativa

Tolerâncias

Precisão de direção: 0.01°

Regras de continuidade

Linhas são C0

Casos especiais

Linhas normais em superfícies com singularidade podem falhar

Boas práticas

Use linhas limitadas para evitar dependências infinitas

1.3 PLANE (Plano)

Inputs

Pontos

Curvas

Direções

Normais

Parâmetros

Offset

Ângulo

Paralelo/perpendicular

Outputs

Plano infinito

Modos de operação

Paramétrico

Não paramétrico

Tolerâncias

Precisão angular: 0.01°

Casos especiais

Planos por três pontos colineares falham

Boas práticas

Prefira planos por ponto + normal para estabilidade

1.4 PROJECTION (Projeção)

Inputs

Curva ou ponto

Superfície ou plano

Direção

Parâmetros

Normal

Direcional

Along vector

Outputs

Curva projetada

Tolerâncias

Tolerância de projeção: 0.001 mm

Casos especiais

Projeções normais podem gerar múltiplas soluções

Boas práticas

Prefira projeção direcional para superfícies complexas

1.5 INTERSECTION (Interseção)

Inputs

Duas superfícies

Superfície + plano

Parâmetros

Exata

Aproximada

Outputs

Curva de interseção

Tolerâncias

Tolerância de interseção: 0.001 mm

Casos especiais

Interseções entre superfícies degeneradas podem falhar

Boas práticas

Use interseção aproximada para superfícies importadas

1.6 CONNECT CURVE

Inputs

Dois pontos ou curvas

Parâmetros

C0, C1, C2

Tension

Outputs

Curva suave

Regras de continuidade

C0: posição

C1: tangência

C2: curvatura

Casos especiais

C2 exige superfícies classe A

Boas práticas

Use C1 para superfícies técnicas

1.7 CORNER

Inputs

Duas curvas

Parâmetros

Raio

Continuidade

Outputs

Curva de canto

Casos especiais

Curvas com ângulo muito agudo podem falhar

Boas práticas

Ajuste o raio conforme a curvatura das curvas

1.8 SPLINE

Inputs

Pontos

Curvas

Direções

Parâmetros

Grau

Tensão

Tangência

Outputs

Curva spline

Casos especiais

Pontos muito próximos podem gerar oscilação

Boas práticas

Use grau 5 para superfícies classe A

2. SURFACES

2.1 EXTRUDE

Inputs

Curva

Direção

Parâmetros

Limites

Direção

Outputs

Superfície extrudada

Tolerâncias

Tolerância de extrusão: 0.001 mm

Casos especiais

Curvas fechadas geram superfícies tubulares

Boas práticas

Prefira direção normal à curva para estabilidade

2.2 REVOLVE

Inputs

Curva

Eixo

Parâmetros

Ângulo

Limites

Outputs

Superfície de revolução

Casos especiais

Curvas que cruzam o eixo geram superfícies degeneradas

Boas práticas

Evite curvas com pontos coincidentes no eixo

2.3 SWEEP

Inputs

Seção

Guia

Spine (opcional)

Parâmetros

Com guia

Com duas guias

Com spine

Com seção

Outputs

Superfície varrida

Tolerâncias

Tolerância de sweep: 0.001 mm

Casos especiais

Guias com curvatura extrema podem gerar torção

Boas práticas

Use spine para controlar torção

2.4 FILL

Inputs

Contorno fechado

Parâmetros

Continuidade C0, C1, C2

Outputs

Superfície preenchida

Casos especiais

Contornos não planos podem gerar rugosidade

Boas práticas

Use C2 apenas para superfícies classe A

2.5 BLEND

Inputs

Duas superfícies

Parâmetros

C0, C1, C2

Raio

Outputs

Superfície de transição

Casos especiais

C2 exige superfícies suaves

Boas práticas

Prefira C1 para superfícies técnicas

3. OPERATIONS

3.1 JOIN

Inputs

Curvas ou superfícies

Parâmetros

Verificar tangência

Verificar curvatura

Outputs

Entidade unificada

Casos especiais

Curvas desconexas não podem ser unidas

Boas práticas

Use Join antes de operações de superfície

3.2 SPLIT

Inputs

Curva, superfície ou sólido

Elemento de corte

Parâmetros

Lado

Manter ambos

Outputs

Elementos divididos

Casos especiais

Splits em superfícies degeneradas podem falhar

Boas práticas

Prefira curvas limpas para cortes

3.3 TRIM

Inputs

Dois elementos

Parâmetros

Lado

Outputs

Elementos aparados

Casos especiais

Trim entre superfícies com interseção parcial pode falhar

Boas práticas

Use interseção antes do trim

4. ADVANCED SURFACES

4.1 FACE FILLET

Inputs

Duas faces

Parâmetros

Raio

Continuidade

Outputs

Filete

Casos especiais

Faces com ângulo muito agudo podem falhar

Boas práticas

Ajuste o raio conforme a curvatura

5. ANALYSIS

5.1 CURVATURE ANALYSIS

Inputs

Superfície

Parâmetros

Tipo de curvatura

Outputs

Mapa de curvatura

Boas práticas

Use zebra para validar continuidade

6. HEALING

6.1 HEALING

Inputs

Superfície

Parâmetros

Gap filling

Boundary healing

Outputs

Superfície corrigida

Casos especiais

Superfícies importadas podem exigir múltiplas correções

Boas práticas

Sempre analise curvatura após healing

7. TRANSFORMATIONS

7.1 TRANSLATE / ROTATE / SCALE / SYMMETRY / AFFINITY

Inputs

Qualquer geometria

Parâmetros

Vetor

Ângulo

Fator

Outputs

Geometria transformada

Boas práticas

Prefira transformações paramétricas

FIM DO DOCUMENTO

Este arquivo contém o detalhamento profissional de todos os comandos do módulo GSD
```
