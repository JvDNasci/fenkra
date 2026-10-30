<!-- origem: Gear toolbox.txt (contexto do usuário, gerado com assistente de IA) -->

```text
Como implementar isso no FEKRA (arquitetura completa)
1️⃣ Criar um “Feature Generator Engine”
Você precisa de um módulo interno responsável por:

ler parâmetros do usuário

gerar geometria paramétrica

atualizar automaticamente quando parâmetros mudam

validar normas (ISO, DIN, AGMA, ABNT)

Esse módulo é o “cérebro” do assistente.

Estrutura recomendada:
Código
/core/parametric_engine/
    gear_generator.cpp
    shaft_generator.cpp
    keyway_generator.cpp
    hole_generator.cpp
    feature_templates/
2️⃣ Implementar “Feature Templates”
Cada tipo de peça é um template paramétrico:

Exemplos:
Engrenagem reta

Engrenagem helicoidal

Engrenagem interna

Cremalheira

Chaveta DIN6885

Chaveta Woodruff

Furo central ISO

Árvore com ombros

Cada template contém:

equações geométricas

restrições paramétricas

regras de geração

validação de normas

3️⃣ Gerar a geometria automaticamente (o coração do sistema)
Para engrenagens:
Você implementa as equações do perfil involuto:

𝑥
=
𝑟
𝑏
(
cos
⁡
𝜃
+
𝜃
sin
⁡
𝜃
)
𝑦
=
𝑟
𝑏
(
sin
⁡
𝜃
−
𝜃
cos
⁡
𝜃
)
Onde:

𝑟
𝑏
 = raio da base

𝜃
 = ângulo paramétrico

Depois:

extrusão

corte dos dentes

filetes

folga

backlash

Para chavetas:
Usar normas:

DIN 6885

ANSI B17.1

ABNT NBR 6158

Para furos:
Normas:

ISO 286

ISO 273

ISO 965 (roscas)

4️⃣ Criar o “Wizard UI” (interface do assistente)
Interface recomendada:
painel lateral com parâmetros

preview dinâmico

presets de normas

botão “Inserir na montagem”

botão “Gerar sólido”

botão “Gerar sketch”

Exemplo de UI:
Código
[Tipo de engrenagem]  Spur / Helical / Internal / Rack
[Módulo]              1.0
[Número de dentes]    24
[Ângulo de pressão]   20°
[Largura]             15 mm
[Backlash]            0.05 mm
[Chaveta]             DIN6885 / Woodruff / Nenhuma
[Furo central]        Ø10 / Rosca M10 / Cônico
[Gerar]               [BOTÃO]
5️⃣ Inserção automática na montagem
Igual ao Fusion e Inventor:

cria mates automáticos

alinha eixo da engrenagem com eixo da árvore

define relação de engrenamento (opcional)

atualiza tudo quando parâmetros mudam

6️⃣ Sistema paramétrico real
Toda peça gerada deve ser:

100% paramétrica

editável no futuro

atualizável sem perder histórico

compatível com o modelador sólido

🧠 Como os outros softwares fazem (para você copiar a estratégia)
Fusion 360
Add-in em Python

Gera involuta via equações

Cria sketch → extrude

Interface simples

Não segue normas industriais (mais hobby)

Inventor (Design Accelerator)
Baseado em normas AGMA

Gera engrenagens completas com tolerâncias

Insere automaticamente na montagem

Atualiza parametricamente

É o mais avançado

SolidWorks Toolbox
Usa tabelas de normas

Gera engrenagens simples

Não é tão avançado quanto Inventor

🛠️ Tecnologias que você precisa no FEKRA
Obrigatório:
Kernel paramétrico (tipo Parasolid, ACIS ou próprio)

Solver geométrico

Sistema de constraints

API para criação de sketches e sólidos

UI dinâmica com preview

Sistema de templates paramétricos

Opcional (mas deixa o FEKRA “premium”):
Simulação de engrenamento

Biblioteca de normas

Geração automática de tolerâncias

Exportação para CAM

🔥 Resumo: como implementar no FEKRA
Criar engine paramétrico

Criar templates de engrenagens, chavetas, furos

Implementar equações involutas

Criar UI de assistente

Gerar sólido automaticamente

Inserir na montagem com mates

Atualizar tudo parametricamente

Se você fizer isso, o FEKRA terá um dos melhores assistentes de geração de peças mecânicas do mercado, superior ao Fusion e comparável ao Inventor.

ser mais proximo do inventor possível
```
