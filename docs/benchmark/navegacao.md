# Benchmark — Navegação

> Texto próprio, a partir das fontes públicas em [fontes.md](fontes.md). Catálogo completo do módulo em
> [catalogo.md](catalogo.md#navegação).

## Como cada ferramenta conduz o fluxo

- **CATIA V5:** Botão do meio arrasta (pan); meio + outro botão gira; clique do meio num ponto recentra a órbita. Alvo estável: nunca "foge" do modelo.
- **SOLIDWORKS:** Meio gira; clique do meio num vértice/aresta/face gira em torno dele; Ctrl+meio = pan; Shift+meio = zoom; roda aproxima no ponteiro.
- **Inventor:** Free Orbit (eixos da tela) e Constrained Orbit (eixo vertical, turntable); o centro pode ser escolhido clicando durante a órbita ou pelo SteeringWheels; ViewCube.

## O que adotar no Fenkra

- CATIA como padrão: alvo fixo + duplo clique do meio recentra por raycast (contexto da câmera).
- Zoom no ponteiro (SW) e turntable opcional (Inventor).

## O que evitar

- Pivô que pula quando não há geometria sob o cursor (usar o centro da caixa como reserva).

## Paridade

| ID | Feature | Fenkra | Falta / observação | Backlog |
|---|---|---|---|---|
| NAV-01 | Órbita em torno de alvo | ✅ | camera.target; estilos de botões por NavStyle. |  |
| NAV-02 | Recentrar no ponto sob o cursor | 🟡 | Só pelo menu do botão direito; sem gesto nem pivô automático sob o cursor. | CTX-CAM |
| NAV-03 | Órbita com eixo vertical (turntable) | ❌ |  | CTX-CAM |
| NAV-04 | Zoom no cursor e zoom por área | 🟡 | Zoom só muda a distância ao alvo. | CTX-CAM |
| NAV-05 | Estilos de navegação | ✅ | NavStyle (settings.rs). |  |
| NAV-06 | Cubo de vistas e vistas nomeadas | ✅ | Cubo e estados de exibição. |  |
