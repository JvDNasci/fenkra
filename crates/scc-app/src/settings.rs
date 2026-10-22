//! Configurações do usuário (persistidas em %APPDATA%\SolidCadCraft\config.json).

use crate::render::{DisplayMode, ViewStyle};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Escuro,
    Claro,
}

/// Grupos de comandos (cada um pode ficar no topo, na lateral, flutuando ou oculto).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Group {
    Sketch,
    SketchDesenho,
    SketchRestricoes,
    Basicos,
    Acabamento,
    Referencias,
    Organizacao,
    Vista,
}

impl Group {
    pub const ALL: [Group; 8] = [
        Group::Sketch,
        Group::SketchDesenho,
        Group::SketchRestricoes,
        Group::Basicos,
        Group::Acabamento,
        Group::Referencias,
        Group::Organizacao,
        Group::Vista,
    ];
    /// Grupos que só aparecem com um sketch em edição.
    pub fn sketch_only(self) -> bool {
        matches!(self, Group::SketchDesenho | Group::SketchRestricoes)
    }
    pub fn label(self) -> &'static str {
        match self {
            Group::Sketch => "Sketch",
            Group::SketchDesenho => "Desenho do sketch",
            Group::SketchRestricoes => "Restrições",
            Group::Basicos => "Básicos",
            Group::Acabamento => "Acabamento",
            Group::Referencias => "Referências",
            Group::Organizacao => "Organização",
            Group::Vista => "Vista",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dock {
    Topo,
    Lateral,
    Flutuante,
    Oculta,
}

impl Dock {
    pub const ALL: [Dock; 4] = [Dock::Topo, Dock::Lateral, Dock::Flutuante, Dock::Oculta];
    pub fn label(self) -> &'static str {
        match self {
            Dock::Topo => "Fixa no topo",
            Dock::Lateral => "Fixa na lateral",
            Dock::Flutuante => "Flutuante",
            Dock::Oculta => "Oculta",
        }
    }
}

/// Filtro de exibição da viewport.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub corpos: bool,
    pub sketches: bool,
    pub pontos: bool,
    pub planos_origem: bool,
    pub planos_ref: bool,
    pub eixos: bool,
    pub rotulos: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Filter { corpos: true, sketches: true, pontos: true, planos_origem: true, planos_ref: true, eixos: true, rotulos: true }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// escala da interface (fontes e controles)
    pub ui_scale: f32,
    pub gradient: bool,
    pub bg_top: [u8; 3],
    pub bg_bottom: [u8; 3],
    pub display: DisplayMode,
    pub edges_in_render: bool,
    pub ortho: bool,
    pub filter: Filter,
    pub docks: Vec<(Group, Dock)>,
    pub icon_size: f32,
    pub show_labels: bool,
    // cotagem (usado pelo sketch e pelo desenho)
    pub dim_standard: String,
    pub dim_decimals: u8,
    pub dim_text_height: f32,
    // sketch
    pub grid: bool,
    pub grid_step: f32,
    pub snaps: bool,
    pub auto_constraints: bool,
    pub auto_dims: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::Escuro,
            ui_scale: 1.0,
            gradient: true,
            bg_top: [74, 84, 100],
            bg_bottom: [26, 29, 34],
            display: DisplayMode::SombreadoArestas,
            edges_in_render: false,
            ortho: false,
            filter: Filter::default(),
            docks: Group::ALL.iter().map(|g| (*g, Dock::Topo)).collect(),
            icon_size: 30.,
            show_labels: true,
            dim_standard: "ABNT NBR 10126".into(),
            dim_decimals: 2,
            dim_text_height: 3.5,
            grid: true,
            grid_step: 10.,
            snaps: true,
            auto_constraints: true,
            auto_dims: false,
        }
    }
}

/// Fundos sugeridos: (nome, topo, base).
pub const BACKGROUNDS: [(&str, [u8; 3], [u8; 3]); 6] = [
    ("Grafite", [74, 84, 100], [26, 29, 34]),
    ("Azul noite", [52, 82, 128], [14, 22, 38]),
    ("Estúdio claro", [236, 240, 246], [178, 188, 204]),
    ("Céu", [176, 206, 236], [238, 242, 246]),
    ("Papel", [250, 250, 248], [250, 250, 248]),
    ("Preto", [0, 0, 0], [0, 0, 0]),
];

impl Settings {
    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("SolidCadCraft").join("config.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default());
        }
    }

    pub fn dock(&self, g: Group) -> Dock {
        self.docks.iter().find(|(x, _)| *x == g).map(|(_, d)| *d).unwrap_or(Dock::Topo)
    }

    pub fn set_dock(&mut self, g: Group, d: Dock) {
        match self.docks.iter_mut().find(|(x, _)| *x == g) {
            Some(e) => e.1 = d,
            None => self.docks.push((g, d)),
        }
    }

    pub fn style(&self) -> ViewStyle {
        ViewStyle {
            mode: self.display,
            bg_top: self.bg_top,
            bg_bottom: self.bg_bottom,
            gradient: self.gradient,
            edges_in_render: self.edges_in_render,
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(match self.theme {
            Theme::Escuro => egui::Visuals::dark(),
            Theme::Claro => egui::Visuals::light(),
        });
        ctx.set_zoom_factor(self.ui_scale.clamp(0.7, 2.0));
    }
}
