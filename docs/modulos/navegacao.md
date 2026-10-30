# Navegação

Estado do módulo no Fenkra (alpha.26). Comparação com outros CADs em [benchmark/navegacao.md](../benchmark/navegacao.md).

## ✅ Pronto

- **Órbita em torno de alvo** (Órbita) — camera.target; estilos de botões por NavStyle.
- **Estilos de navegação** (Estilos: Fenkra, SolidWorks, Fusion, CATIA, Blender, Onshape, FreeCAD, Inventor, Rhino) — NavStyle (settings.rs).
- **Cubo de vistas e vistas nomeadas** (Cubo de vistas, estados de exibição) — Cubo e estados de exibição.

## 🟡 Parcial

- **Recentrar no ponto sob o cursor** (Menu "Centro de órbita aqui") — Só pelo menu do botão direito; sem gesto nem pivô automático sob o cursor. `CTX-CAM`
- **Zoom no cursor e zoom por área** (Zoom (distância)) — Zoom só muda a distância ao alvo. `CTX-CAM`

## ❌ Ainda não existe

- **Órbita com eixo vertical (turntable)** (—) —  `CTX-CAM`
