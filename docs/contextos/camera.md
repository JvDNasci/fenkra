<!-- origem: Camera feature.txt (contexto do usuário, gerado com assistente de IA) -->

```text
COMO CADA SOFTWARE FAZ A MOVIMENTAÇÃO DE CÂMERA
Abaixo está a análise detalhada que você pediu.

🟦 CATIA V5 — O padrão que você quer copiar
🎯 Característica principal:
Orbit sempre em torno do ponto selecionado ou do centro do modelo.

🧠 Como funciona:
CATIA define um Target Point fixo.

A câmera gira ao redor desse ponto usando:

Rotação em torno do eixo X local

Rotação em torno do eixo Y local

O Target nunca muda a menos que o usuário:

selecione outro ponto

dê um “Fit All”

use “Recenter”

🌀 Sensação:
Extremamente estável

Nunca “escapa” do modelo

Ideal para CAD mecânico

📌 Implementação no FEKRA:
Código
camera.target = selected_point_or_center;
camera.rotateAround(camera.target, deltaX, deltaY);
🟨 SolidWorks — Orbit com foco no cursor
🎯 Característica:
Orbit baseado no ponto sob o cursor.

🧠 Como funciona:
Quando o usuário segura o botão do mouse:

SolidWorks faz um raycast

Encontra o ponto 3D sob o cursor

Usa esse ponto como novo target

🌀 Sensação:
Muito intuitivo

Ótimo para peças pequenas

Pode “pular” se não houver geometria sob o cursor

📌 Implementação no FEKRA:
Código
target = raycast(cursor_position);
camera.rotateAround(target);
🟧 Fusion 360 — Orbit livre + pivot inteligente
🎯 Característica:
Pivot automático baseado na geometria mais próxima.

🧠 Como funciona:
Fusion calcula o ponto mais próximo da câmera

Usa esse ponto como pivot

Se não houver geometria, usa o centro da cena

🌀 Sensação:
Suave

Cinemático

Bom para modelagem orgânica

📌 Implementação no FEKRA:
Código
target = nearest_geometry_point(camera.position);
camera.rotateAround(target);
🟩 FreeCAD — Orbit clássico estilo CAD antigo
🎯 Característica:
Orbit fixo no centro da vista.

🧠 Como funciona:
Target fixo no centro da bounding box

Não muda automaticamente

🌀 Sensação:
Estável

Menos intuitivo

📌 Implementação no FEKRA:
Código
target = model.boundingBox.center;
camera.rotateAround(target);
🟪 Blender — Orbit da câmera, não do modelo
🎯 Característica:
Blender gira a câmera em si, não o modelo.

🧠 Como funciona:
A câmera gira em torno do próprio eixo

O target é opcional (Track To constraint)

🌀 Sensação:
Cinemático

Não ideal para CAD mecânico

📌 Implementação no FEKRA:
Código
camera.rotateSelf(deltaX, deltaY);
🟥 Inventor — Turntable + Orbit híbrido
🎯 Característica:
Orbit com eixo fixo vertical.

🧠 Como funciona:
Rotação horizontal sempre no eixo Z global

Rotação vertical no eixo X local

🌀 Sensação:
Muito estável

Ótimo para montagens grandes

📌 Implementação no FEKRA:
Código
camera.rotateAround(target, yaw_globalZ, pitch_localX);
🟫 Rhino — Orbit livre com foco no cursor
🎯 Característica:
Orbit semelhante ao SolidWorks.

🧠 Como funciona:
Raycast no cursor

Pivot dinâmico

🌀 Sensação:
Muito preciso

Bom para superfícies

⭐ COMO IMPLEMENTAR O SISTEMA DE CÂMERA NO FEKRA (O QUE VOCÊ REALMENTE QUER)
Você quer que o FEKRA tenha:

Orbit estilo CATIA V5 como padrão

Opções de estilo SolidWorks, Fusion, Inventor, etc.

Igual ao menu que você mostrou na imagem

Então vamos montar o sistema.

🧩 ARQUITETURA DO SISTEMA DE CÂMERA DO FEKRA
1. Estrutura da câmera
Código
Camera {
    Vector3 position;
    Vector3 target;
    Vector3 up;
    float fov;
}
2. Modos de orbit
Código
enum OrbitMode {
    CATIA,
    SOLIDWORKS,
    FUSION,
    INVENTOR,
    FREECAD,
    BLENDER,
    RHINO,
    ONSHAPE
}
3. Função principal
Código
void orbit(deltaX, deltaY) {
    switch (orbitMode) {
        case CATIA:
            orbitAroundFixedTarget(deltaX, deltaY);
            break;
        case SOLIDWORKS:
            orbitAroundCursorRaycast(deltaX, deltaY);
            break;
        case FUSION:
            orbitAroundNearestGeometry(deltaX, deltaY);
            break;
        case INVENTOR:
            orbitTurntable(deltaX, deltaY);
            break;
        case BLENDER:
            orbitCameraSelf(deltaX, deltaY);
            break;
        ...
    }
}
🔥 POR QUE O ORBITA DO CATIA DEVE SER PADRÃO NO FEKRA
Porque:

é o mais estável

nunca perde o modelo

é perfeito para CAD mecânico

é o mais usado em indústria pesada (aeronáutica, automotiva)

evita frustração de usuário iniciante

SolidWorks e Fusion são ótimos, mas o CATIA é o mais profissional.


mais rápido que o Shift + Scroll.

Vamos destrinchar isso com precisão cirúrgica.

🎯 CATIA V5 — Duplo clique no scroll = Recenter automático
No CATIA V5, o gesto:

🖱️ Duplo clique no scroll (botão do meio)
executa o comando:

🔵 Recenter View (redefinir o pivot da órbita)
Ou seja:

CATIA faz um raycast no ponto da geometria onde o cursor está

Esse ponto vira o novo centro da órbita

A câmera passa a girar exatamente ao redor desse ponto

É o mesmo efeito do Shift + Middle Mouse Button, só que mais rápido

🧠 Como o CATIA interpreta esse gesto
Quando você dá duplo clique no scroll, o CATIA faz:

Verifica se o cursor está sobre geometria

Se estiver:

calcula o ponto 3D exato

define esse ponto como camera.target

Se não estiver:

usa o centro da bounding box como fallback

Isso é o que faz o CATIA ser tão estável e previsível.

🔥 Por que isso é importante para o FEKRA
Se você quer que o FEKRA tenha navegação estilo CATIA como padrão, você precisa implementar exatamente esse comportamento:

✔ Duplo clique no scroll → Recenter
Raycast no cursor

Define novo pivot

Atualiza a órbita imediatamente

✔ Scroll + arrastar → Orbit
✔ Scroll + Shift → Pan
✔ Scroll + Ctrl → Zoom suave
Esse conjunto é o que faz o CATIA ser tão “profissional” na navegação.

🧩 Implementação no FEKRA (lógica exata)
Evento:
Código
onMiddleMouseDoubleClick(cursorPosition):
    point3D = raycast(cursorPosition)
    if point3D exists:
        camera.target = point3D
    else:
        camera.target = model.boundingBox.center
Resultado:
Orbit gira ao redor do novo target

Pan e zoom continuam relativos ao target

Navegação fica idêntica ao CATIA

🎯 Resumo rápido
Ação	Gesto	Efeito
Recenter	Duplo clique no scroll	Define novo centro da órbita
Recenter manual	Shift + scroll	Mesmo efeito, versão “oficial”
Orbit	Scroll + arrastar	Gira ao redor do target
Pan	Shift + arrastar	Move câmera e target juntos
Zoom	Ctrl + scroll	Aproxima/afasta

```
