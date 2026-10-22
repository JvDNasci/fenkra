//! Atalhos de teclado configuráveis, com predefinições inspiradas nos atalhos padrão de outros
//! CADs (SolidWorks, CATIA, FreeCAD, Fusion, Blender, Onshape, Inventor). Cada predefinição também
//! escolhe o estilo de navegação do mouse; qualquer atalho pode ser trocado individualmente.

use crate::settings::NavStyle;
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum KeyAction {
    // vista
    Fit,
    ViewMenu,
    Normal,
    Ortho,
    ViewFront,
    ViewBack,
    ViewTop,
    ViewBottom,
    ViewLeft,
    ViewRight,
    ViewIso,
    Hide,
    HiddenEnv,
    // edição
    Undo,
    Redo,
    Delete,
    Save,
    // modelagem
    Sketch,
    Extrude,
    Cut,
    Revolve,
    Hole,
    Fillet,
    Chamfer,
    Plane,
    // sketch
    SkSelect,
    SkLine,
    SkRect,
    SkCircle,
    SkArc,
    SkPoint,
    SkDim,
    SkConstruction,
    SkGrid,
}

use KeyAction::*;

impl KeyAction {
    pub const ALL: [KeyAction; 34] = [
        Fit, ViewMenu, Normal, Ortho, ViewFront, ViewBack, ViewTop, ViewBottom, ViewLeft, ViewRight, ViewIso, Hide, HiddenEnv, Undo, Redo, Delete, Save, Sketch,
        Extrude, Cut, Revolve, Hole, Fillet, Chamfer, Plane, SkSelect, SkLine, SkRect, SkCircle, SkArc, SkPoint, SkDim, SkConstruction, SkGrid,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Fit => "Ajustar à tela",
            ViewMenu => "Seletor de vistas",
            Normal => "Normal à seleção",
            Ortho => "Ortogonal / perspectiva",
            ViewFront => "Vista frontal",
            ViewBack => "Vista traseira",
            ViewTop => "Vista superior",
            ViewBottom => "Vista inferior",
            ViewLeft => "Vista esquerda",
            ViewRight => "Vista direita",
            ViewIso => "Vista isométrica",
            Hide => "Ocultar/mostrar seleção",
            HiddenEnv => "Ambiente de ocultos",
            Undo => "Desfazer",
            Redo => "Refazer",
            Delete => "Apagar",
            Save => "Salvar",
            Sketch => "Novo sketch",
            Extrude => "Extrusão",
            Cut => "Recorte",
            Revolve => "Revolução",
            Hole => "Furo",
            Fillet => "Filete",
            Chamfer => "Chanfro",
            Plane => "Plano",
            SkSelect => "Sketch · selecionar",
            SkLine => "Sketch · linha",
            SkRect => "Sketch · retângulo",
            SkCircle => "Sketch · círculo",
            SkArc => "Sketch · arco",
            SkPoint => "Sketch · ponto",
            SkDim => "Sketch · cota",
            SkConstruction => "Sketch · construção",
            SkGrid => "Sketch · grade",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Fit | ViewMenu | Normal | Ortho | ViewFront | ViewBack | ViewTop | ViewBottom | ViewLeft | ViewRight | ViewIso | Hide | HiddenEnv => "Vista",
            Undo | Redo | Delete | Save => "Edição",
            Sketch | Extrude | Cut | Revolve | Hole | Fillet | Chamfer | Plane => "Modelagem",
            _ => "Sketch",
        }
    }

    /// Ações que só valem dentro do sketch.
    pub fn sketch_only(self) -> bool {
        self.group() == "Sketch"
    }
}

/// Combinação de teclas ("Ctrl+Shift+Z", "F6", "Space").
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Binding {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Binding {
    pub fn parse(s: &str) -> Option<Binding> {
        let mut b = Binding::default();
        for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" => b.ctrl = true,
                "shift" => b.shift = true,
                "alt" => b.alt = true,
                _ => b.key = part.to_string(),
            }
        }
        egui::Key::from_name(&b.key).map(|_| b)
    }

    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s += "Ctrl+";
        }
        if self.shift {
            s += "Shift+";
        }
        if self.alt {
            s += "Alt+";
        }
        s + &self.key
    }

    pub fn pressed(&self, i: &egui::InputState) -> bool {
        let Some(k) = egui::Key::from_name(&self.key) else { return false };
        let m = i.modifiers;
        i.key_pressed(k) && m.command == self.ctrl && m.shift == self.shift && m.alt == self.alt
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Preset {
    #[default]
    SolidCadCraft,
    SolidWorks,
    Catia,
    FreeCad,
    Fusion,
    Blender,
    Onshape,
    Inventor,
}

impl Preset {
    pub const ALL: [Preset; 8] = [Preset::SolidCadCraft, Preset::SolidWorks, Preset::Catia, Preset::FreeCad, Preset::Fusion, Preset::Blender, Preset::Onshape, Preset::Inventor];

    pub fn label(self) -> &'static str {
        match self {
            Preset::SolidCadCraft => "SolidCadCraft",
            Preset::SolidWorks => "SolidWorks",
            Preset::Catia => "CATIA V5",
            Preset::FreeCad => "FreeCAD",
            Preset::Fusion => "Fusion 360",
            Preset::Blender => "Blender",
            Preset::Onshape => "Onshape",
            Preset::Inventor => "Inventor",
        }
    }

    /// Navegação do mouse que acompanha a predefinição.
    pub fn nav(self) -> NavStyle {
        match self {
            Preset::SolidCadCraft => NavStyle::SolidCadCraft,
            Preset::SolidWorks => NavStyle::SolidWorks,
            Preset::Catia => NavStyle::Catia,
            Preset::FreeCad => NavStyle::FreeCad,
            Preset::Fusion => NavStyle::Fusion,
            Preset::Blender => NavStyle::Blender,
            Preset::Onshape => NavStyle::Onshape,
            Preset::Inventor => NavStyle::Inventor,
        }
    }

    /// Atalhos da predefinição (o que não aparece fica sem atalho).
    pub fn defaults(self) -> Vec<(KeyAction, &'static str)> {
        // comuns a quase todos
        let mut v = vec![(Undo, "Ctrl+Z"), (Redo, "Ctrl+Y"), (Delete, "Delete"), (Save, "Ctrl+S"), (ViewMenu, "Space")];
        let sk = [(SkSelect, "S"), (SkLine, "L"), (SkRect, "R"), (SkCircle, "C"), (SkArc, "A"), (SkPoint, "P"), (SkDim, "D"), (SkConstruction, "X"), (SkGrid, "G")];
        match self {
            Preset::SolidCadCraft => {
                v.extend([(Fit, "F"), (Normal, "N"), (Ortho, "O"), (ViewFront, "Ctrl+1"), (ViewBack, "Ctrl+2"), (ViewRight, "Ctrl+3"), (ViewLeft, "Ctrl+4"), (ViewTop, "Ctrl+5"), (ViewBottom, "Ctrl+6"), (ViewIso, "Ctrl+7"), (Hide, "H"), (HiddenEnv, "Shift+H"), (Extrude, "E"), (Revolve, "Shift+R"), (Fillet, "Shift+F"), (Chamfer, "Shift+C"), (Sketch, "Shift+S"), (Plane, "Shift+P")]);
                v.extend(sk);
            }
            Preset::SolidWorks => {
                // Ctrl+1..8 = frente, trás, esquerda, direita, topo, base, iso, normal a
                v.extend([(Fit, "F"), (ViewFront, "Ctrl+1"), (ViewBack, "Ctrl+2"), (ViewLeft, "Ctrl+3"), (ViewRight, "Ctrl+4"), (ViewTop, "Ctrl+5"), (ViewBottom, "Ctrl+6"), (ViewIso, "Ctrl+7"), (Normal, "Ctrl+8"), (Hide, "Tab"), (HiddenEnv, "Shift+Tab"), (Ortho, "O")]);
                v.extend([(SkSelect, "S"), (SkLine, "L"), (SkDim, "D"), (SkRect, "R"), (SkCircle, "C"), (SkArc, "A"), (SkPoint, "P"), (SkConstruction, "X"), (SkGrid, "G")]);
            }
            Preset::Catia => {
                // CATIA quase não tem atalhos de fábrica: vistas pelos números e o resto como no SolidCadCraft
                v.extend([(Fit, "F"), (Normal, "N"), (ViewFront, "Ctrl+1"), (ViewBack, "Ctrl+2"), (ViewRight, "Ctrl+3"), (ViewLeft, "Ctrl+4"), (ViewTop, "Ctrl+5"), (ViewBottom, "Ctrl+6"), (ViewIso, "Ctrl+7"), (Hide, "H"), (HiddenEnv, "Shift+H"), (Ortho, "O")]);
                v.extend(sk);
            }
            Preset::FreeCad => {
                // 0 iso, 1 frente, 2 topo, 3 direita, 4 trás, 5 base, 6 esquerda; Espaço oculta
                v.retain(|(a, _)| *a != ViewMenu);
                v.extend([(ViewIso, "0"), (ViewFront, "1"), (ViewTop, "2"), (ViewRight, "3"), (ViewBack, "4"), (ViewBottom, "5"), (ViewLeft, "6"), (Fit, "V"), (Ortho, "O"), (Hide, "Space"), (ViewMenu, "Shift+Space"), (HiddenEnv, "Shift+H"), (Normal, "N")]);
                v.extend(sk);
            }
            Preset::Fusion => {
                // F6 ajusta; atalhos de comando de uma letra
                v.extend([(Fit, "F6"), (ViewIso, "Home"), (Hide, "V"), (HiddenEnv, "Shift+V"), (Normal, "N"), (Ortho, "O"), (Extrude, "E"), (Fillet, "F"), (Hole, "H"), (Sketch, "Shift+S")]);
                v.extend([(SkLine, "L"), (SkRect, "R"), (SkCircle, "C"), (SkArc, "A"), (SkPoint, "P"), (SkDim, "D"), (SkConstruction, "X"), (SkSelect, "S"), (SkGrid, "G")]);
            }
            Preset::Blender => {
                // teclado numérico: 1 frente, 3 direita, 7 topo; Ctrl inverte; 5 orto; Home ajusta
                v.retain(|(a, _)| *a != Redo && *a != ViewMenu);
                v.extend([(Redo, "Ctrl+Shift+Z"), (ViewMenu, "`"), (ViewFront, "1"), (ViewBack, "Ctrl+1"), (ViewRight, "3"), (ViewLeft, "Ctrl+3"), (ViewTop, "7"), (ViewBottom, "Ctrl+7"), (ViewIso, "9"), (Ortho, "5"), (Fit, "Home"), (Hide, "H"), (HiddenEnv, "Alt+H"), (Normal, "N"), (Extrude, "E")]);
                v.extend(sk);
            }
            Preset::Onshape => {
                // Shift+1..7 vistas; F ajusta; N normal
                v.extend([(Fit, "F"), (Normal, "N"), (ViewFront, "Shift+1"), (ViewBack, "Shift+2"), (ViewLeft, "Shift+3"), (ViewRight, "Shift+4"), (ViewTop, "Shift+5"), (ViewBottom, "Shift+6"), (ViewIso, "Shift+7"), (Hide, "Y"), (HiddenEnv, "Shift+Y"), (Ortho, "O"), (Extrude, "Shift+E"), (Sketch, "Shift+S")]);
                v.extend([(SkLine, "L"), (SkRect, "R"), (SkCircle, "C"), (SkArc, "A"), (SkPoint, "P"), (SkDim, "D"), (SkConstruction, "Q"), (SkSelect, "S"), (SkGrid, "G")]);
            }
            Preset::Inventor => {
                // F6 casa (iso), Home ajusta; E extrusão, R revolução, H furo, S sketch
                v.extend([(ViewIso, "F6"), (Fit, "Home"), (Normal, "N"), (Ortho, "O"), (Hide, "Alt+V"), (HiddenEnv, "Shift+H"), (Extrude, "E"), (Revolve, "R"), (Hole, "H"), (Sketch, "S"), (Fillet, "Shift+F"), (Chamfer, "Ctrl+Shift+K")]);
                v.extend([(SkLine, "L"), (SkCircle, "C"), (SkArc, "A"), (SkPoint, "P"), (SkDim, "D"), (SkRect, "R"), (SkConstruction, "X"), (SkSelect, "Escape"), (SkGrid, "G")]);
                v.retain(|(a, k)| !(*a == SkSelect && *k == "Escape"));
            }
        }
        v
    }
}

/// Mapa de atalhos: predefinição + trocas feitas pelo usuário.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Keymap {
    pub preset: Preset,
    /// (ação, atalho) trocados pelo usuário; atalho vazio = sem atalho
    pub custom: Vec<(KeyAction, String)>,
}

impl Keymap {
    pub fn binding(&self, a: KeyAction) -> Option<Binding> {
        if let Some((_, s)) = self.custom.iter().find(|(x, _)| *x == a) {
            return Binding::parse(s);
        }
        self.preset.defaults().into_iter().find(|(x, _)| *x == a).and_then(|(_, s)| Binding::parse(s))
    }

    pub fn set(&mut self, a: KeyAction, b: Option<Binding>) {
        self.custom.retain(|(x, _)| *x != a);
        self.custom.push((a, b.map(|b| b.text()).unwrap_or_default()));
    }

    pub fn pressed(&self, ctx: &egui::Context, a: KeyAction) -> bool {
        self.binding(a).is_some_and(|b| ctx.input(|i| b.pressed(i)))
    }

    /// Outras ações (no mesmo contexto) que usam o mesmo atalho.
    pub fn conflicts(&self, a: KeyAction) -> Vec<KeyAction> {
        let Some(b) = self.binding(a) else { return vec![] };
        KeyAction::ALL.iter().copied().filter(|x| *x != a && x.sketch_only() == a.sketch_only() && self.binding(*x).as_ref() == Some(&b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predefinicoes_validas_e_sem_conflitos() {
        for p in Preset::ALL {
            for (a, k) in p.defaults() {
                assert!(Binding::parse(k).is_some(), "{p:?}: atalho inválido {k} para {a:?}");
            }
            let km = Keymap { preset: p, custom: vec![] };
            for a in KeyAction::ALL {
                assert!(km.conflicts(a).is_empty(), "{p:?}: {a:?} em conflito com {:?}", km.conflicts(a));
            }
        }
        assert_eq!(Binding::parse("Ctrl+Shift+Z").unwrap().text(), "Ctrl+Shift+Z");
    }
}
