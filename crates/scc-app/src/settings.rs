//! Configurações do usuário (persistidas em %APPDATA%\SolidCadCraft\config.json).

use crate::render::{DisplayMode, ViewStyle};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Escuro,
    Claro,
}

/// Grupos de comandos (blocos da barra). Cada um pode ficar no topo, na lateral, flutuando ou oculto.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Group {
    Sketch,
    Solidos,
    Acabamento,
    Referencias,
    Organizacao,
    Selecao,
    Vista,
    Padroes,
    // bancada de superfícies
    /// criação de pontos, linhas, planos e curvas à mão (bancada de superfícies)
    Wireframe,
    Superficies,
    Operacoes,
    /// comandos híbridos superfície ↔ sólido (nas duas bancadas)
    Hibrido,
    /// análise de superfície (zebra, curvatura) — nas duas bancadas
    Analise,
    // montagem
    Montagem,
    Restricoes,
    Cinematica,
    /// chapa metálica
    Chapa,
    // modo sketch
    SkCriacao,
    SkTransformar,
    SkCotagem,
    SkRestricoes,
    SkFerramentas,
}

impl Group {
    pub const ALL: [Group; 22] = [
        Group::Chapa,
        Group::Montagem,
        Group::Restricoes,
        Group::Cinematica,
        Group::Sketch,
        Group::Solidos,
        Group::Acabamento,
        Group::Padroes,
        Group::Wireframe,
        Group::Superficies,
        Group::Operacoes,
        Group::Hibrido,
        Group::Referencias,
        Group::Analise,
        Group::Organizacao,
        Group::Selecao,
        Group::Vista,
        Group::SkCriacao,
        Group::SkTransformar,
        Group::SkCotagem,
        Group::SkRestricoes,
        Group::SkFerramentas,
    ];
    /// Em que bancada o grupo aparece (Sketch, Referências/wireframe, Volume, Análise, Organização,
    /// Seleção e Vista são comuns às duas).
    pub fn in_bench(self, b: Bench) -> bool {
        if b == Bench::Chapa {
            return matches!(self, Group::Sketch | Group::Chapa | Group::Referencias | Group::Organizacao | Group::Vista | Group::Selecao | Group::Analise);
        }
        if self == Group::Chapa {
            return false;
        }
        if b == Bench::Montagem {
            return matches!(self, Group::Montagem | Group::Restricoes | Group::Cinematica | Group::Vista | Group::Selecao | Group::Analise);
        }
        match self {
            Group::Montagem | Group::Restricoes | Group::Cinematica => false,
            Group::Solidos | Group::Acabamento | Group::Padroes => b == Bench::Solidos,
            Group::Superficies | Group::Operacoes | Group::Wireframe => b == Bench::Superficies,
            // na bancada de superfícies o Wireframe mostra as referências abertas, sem famílias
            Group::Referencias => b == Bench::Solidos,
            _ => true,
        }
    }
    /// Grupos que só aparecem com um sketch em edição.
    pub fn sketch_only(self) -> bool {
        matches!(self, Group::SkCriacao | Group::SkTransformar | Group::SkCotagem | Group::SkRestricoes | Group::SkFerramentas)
    }
    pub fn label(self) -> &'static str {
        match self {
            Group::Sketch => tr("Sketch"),
            Group::Solidos => tr("Sólidos"),
            Group::Acabamento => tr("Acabamento"),
            Group::Referencias => tr("Referências"),
            Group::Organizacao => tr("Organização"),
            Group::Selecao => tr("Seleção"),
            Group::Vista => tr("Vista"),
            Group::Padroes => tr("Padrões e transformações"),
            Group::Superficies => tr("Superfícies"),
            Group::Wireframe => tr("Wireframe (pontos, linhas, planos, curvas)"),
            Group::Operacoes => tr("Operações"),
            Group::Hibrido => tr("Volume (superfície ↔ sólido)"),
            Group::Analise => tr("Análise"),
            Group::Montagem => tr("Montagem"),
            Group::Restricoes => tr("Restrições e movimento"),
            Group::Cinematica => tr("Cinemática"),
            Group::Chapa => tr("Chapa metálica"),
            Group::SkCriacao => tr("Criação"),
            Group::SkTransformar => tr("Transformar"),
            Group::SkCotagem => tr("Cotagem"),
            Group::SkRestricoes => tr("Restrições"),
            Group::SkFerramentas => tr("Ferramentas do sketch"),
        }
    }
}

/// Bancada (módulo) ativa: muda os grupos de comandos; o modelo é o mesmo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Bench {
    #[default]
    Solidos,
    Superficies,
    Montagem,
    Chapa,
}

/// Estilo de navegação da câmera (botões do mouse).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum NavStyle {
    #[default]
    SolidCadCraft,
    SolidWorks,
    Fusion,
    Catia,
    Blender,
    Onshape,
    FreeCad,
    Inventor,
    Rhino,
}

impl NavStyle {
    pub const ALL: [NavStyle; 9] = [
        NavStyle::SolidCadCraft,
        NavStyle::SolidWorks,
        NavStyle::Fusion,
        NavStyle::Catia,
        NavStyle::FreeCad,
        NavStyle::Blender,
        NavStyle::Onshape,
        NavStyle::Inventor,
        NavStyle::Rhino,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NavStyle::SolidCadCraft => tr("SolidCadCraft"),
            NavStyle::SolidWorks => tr("Estilo SolidWorks"),
            NavStyle::Fusion => tr("Estilo Fusion"),
            NavStyle::Catia => tr("Estilo CATIA"),
            NavStyle::Blender => tr("Estilo Blender"),
            NavStyle::Onshape => tr("Estilo Onshape"),
            NavStyle::FreeCad => tr("Estilo FreeCAD (CAD)"),
            NavStyle::Inventor => tr("Estilo Inventor"),
            NavStyle::Rhino => tr("Estilo Rhino"),
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            NavStyle::SolidCadCraft => tr("Girar: botão esquerdo ou do meio · Mover: direito ou Shift+arrastar · Zoom: roda"),
            NavStyle::SolidWorks => tr("Girar: botão do meio · Mover: Ctrl+meio · Zoom: roda (invertida)"),
            NavStyle::Fusion => tr("Girar: Shift+meio · Mover: botão do meio · Zoom: roda"),
            NavStyle::Catia => tr("Mover: botão do meio · Girar: meio+direito · Zoom: roda"),
            NavStyle::Blender => tr("Girar: botão do meio · Mover: Shift+meio · Zoom: roda ou Ctrl+meio"),
            NavStyle::Onshape => tr("Girar: botão direito · Mover: Ctrl+direito ou meio · Zoom: roda"),
            NavStyle::FreeCad => tr("Mover: botão do meio · Girar: meio + esquerdo ou direito · Zoom: roda"),
            NavStyle::Inventor => tr("Girar: Shift+meio · Mover: botão do meio · Zoom: roda"),
            NavStyle::Rhino => tr("Girar: botão direito · Mover: Shift+direito · Zoom: roda ou Ctrl+direito"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dock {
    Topo,
    Lateral,
    Flutuante,
    Oculta,
    /// fixa na lateral direita
    Direita,
}

impl Dock {
    pub const ALL: [Dock; 5] = [Dock::Topo, Dock::Lateral, Dock::Direita, Dock::Flutuante, Dock::Oculta];
    pub fn label(self) -> &'static str {
        match self {
            Dock::Topo => tr("Fixa no topo"),
            Dock::Lateral => tr("Fixa na lateral esquerda"),
            Dock::Direita => tr("Fixa na lateral direita"),
            Dock::Flutuante => tr("Flutuante"),
            Dock::Oculta => tr("Oculta"),
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
    /// variação escolhida em cada família de comandos (chave → índice)
    pub families: Vec<(String, usize)>,
    pub nav: NavStyle,
    pub invert_zoom: bool,
    /// filtro de seleção na viewport
    pub sel_faces: bool,
    pub sel_edges: bool,
    pub sel_vertices: bool,
    /// projetos abertos recentemente (pastas)
    pub recent: Vec<String>,
    pub show_start: bool,
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
    /// atalhos de teclado (predefinição + trocas)
    pub keys: crate::keymap::Keymap,
    /// interface em inglês
    pub english: bool,
    /// personalização dos grupos de comandos
    pub custom: Customization,
    /// bancada ativa (Modelagem Sólida ou Superfícies)
    #[serde(default)]
    pub bench: Bench,
    /// recálculo automático a cada mudança (desligado: só com "Atualizar")
    pub auto_update: bool,
    /// painel de edição à esquerda (junto da árvore) em vez da direita
    pub editor_left: bool,
    /// última macro escrita na janela de automação
    pub macro_src: String,
}

/// Comandos ocultos, ordem própria em cada grupo e grupos criados pelo usuário.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Customization {
    /// chaves das entradas escondidas
    pub hidden: Vec<String>,
    /// ordem das entradas por grupo (chaves)
    pub order: Vec<(Group, Vec<String>)>,
    pub groups: Vec<CustomGroup>,
}

/// Grupo de comandos montado pelo usuário (fica na barra de cima).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomGroup {
    pub name: String,
    pub items: Vec<String>,
    /// aparece no modo sketch (senão, na modelagem)
    pub sketch: bool,
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
            families: vec![],
            nav: NavStyle::SolidCadCraft,
            invert_zoom: false,
            sel_faces: true,
            sel_edges: true,
            sel_vertices: true,
            recent: vec![],
            show_start: true,
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
            keys: Default::default(),
            english: false,
            custom: Default::default(),
            macro_src: String::new(),
            bench: Default::default(),
            auto_update: true,
            editor_left: true,
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

    pub fn family(&self, key: &str) -> usize {
        self.families.iter().find(|(k, _)| k == key).map(|(_, i)| *i).unwrap_or(0)
    }
    pub fn set_family(&mut self, key: &str, i: usize) {
        match self.families.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = i,
            None => self.families.push((key.to_string(), i)),
        }
    }
    pub fn add_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(8);
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
