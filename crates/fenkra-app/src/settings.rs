//! Configurações do usuário (persistidas em %APPDATA%\Fenkra\config.json).

use crate::render::{DisplayMode, Msaa, ViewStyle};
use eframe::egui;
use fenkra_core::{PresetMalha, QualidadeMalha};
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
    /// membros estruturais
    Estrutural,
    /// detalhamento 3D (PMI)
    Pmi,
    // detalhamento 2D
    DesArquivo,
    DesVistas,
    DesCotas,
    DesAnotacoes,
    DesTabelas,
    // modo sketch
    SkCriacao,
    SkTransformar,
    SkCotagem,
    SkRestricoes,
    SkFerramentas,
}

impl Group {
    pub const ALL: [Group; 29] = [
        Group::DesArquivo,
        Group::DesVistas,
        Group::DesCotas,
        Group::DesAnotacoes,
        Group::DesTabelas,
        Group::Chapa,
        Group::Estrutural,
        Group::Pmi,
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
        let des = matches!(self, Group::DesArquivo | Group::DesVistas | Group::DesCotas | Group::DesAnotacoes | Group::DesTabelas);
        if b == Bench::Desenho || des {
            return b == Bench::Desenho && des;
        }
        if b == Bench::Chapa {
            return matches!(self, Group::Sketch | Group::Chapa | Group::Referencias | Group::Organizacao | Group::Pmi | Group::Vista | Group::Selecao | Group::Analise);
        }
        if b == Bench::Estrutural {
            return matches!(self, Group::Sketch | Group::Estrutural | Group::Referencias | Group::Organizacao | Group::Pmi | Group::Vista | Group::Selecao | Group::Analise);
        }
        if matches!(self, Group::Chapa | Group::Estrutural) {
            return false;
        }
        if b == Bench::Montagem {
            return matches!(self, Group::Montagem | Group::Restricoes | Group::Cinematica | Group::Pmi | Group::Vista | Group::Selecao | Group::Analise);
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
    /// O grupo é do trabalho atual e fica à vista na faixa; os demais vão para o botão "Mais".
    pub fn do_contexto(self, b: Bench, sketching: bool) -> bool {
        if sketching {
            return self.sketch_only();
        }
        match b {
            Bench::Solidos => matches!(self, Group::Sketch | Group::Solidos | Group::Acabamento | Group::Padroes),
            Bench::Superficies => matches!(self, Group::Sketch | Group::Wireframe | Group::Superficies | Group::Operacoes),
            Bench::Montagem => matches!(self, Group::Montagem | Group::Restricoes),
            Bench::Chapa => matches!(self, Group::Sketch | Group::Chapa),
            Bench::Estrutural => matches!(self, Group::Sketch | Group::Estrutural),
            Bench::Desenho => matches!(self, Group::DesVistas | Group::DesCotas | Group::DesAnotacoes),
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
            Group::Estrutural => tr("Estrutural"),
            Group::Pmi => tr("Detalhamento 3D (PMI)"),
            Group::DesArquivo => tr("Desenho"),
            Group::DesVistas => tr("Vistas"),
            Group::DesCotas => tr("Cotas"),
            Group::DesAnotacoes => tr("Anotações e símbolos"),
            Group::DesTabelas => tr("Tabelas"),
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
    Estrutural,
    /// detalhamento 2D (módulo separado: arquivo .fkd)
    Desenho,
}

/// Família de perfis cadastrada: tipo de seção, tamanhos e a regra de criação (ancoragem e rotação
/// padrão quando o membro nasce do arame 3D).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UserFamily {
    pub name: String,
    pub kind: fenkra_core::SectionKind,
    pub sizes: Vec<(String, Vec<f64>)>,
    #[serde(default)]
    pub anchor: u8,
    #[serde(default)]
    pub angle: f64,
}

/// Estilo de navegação da câmera (botões do mouse).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NavStyle {
    #[default]
    Fenkra,
    /// Meio gira, Ctrl+meio move, zoom invertido
    MeioGira,
    /// Shift+meio gira, meio move (absorve dois nomes antigos)
    ShiftMeioGira,
    /// Meio move, meio+direito gira
    MeioMove,
    Blender,
    /// Direito gira, Ctrl+direito ou meio move
    DireitoGira,
    FreeCad,
    /// Direito gira, Shift+direito move, Ctrl+direito dá zoom
    DireitoGiraShift,
}

impl NavStyle {
    pub const ALL: [NavStyle; 8] = [
        NavStyle::Fenkra,
        NavStyle::MeioGira,
        NavStyle::ShiftMeioGira,
        NavStyle::MeioMove,
        NavStyle::FreeCad,
        NavStyle::Blender,
        NavStyle::DireitoGira,
        NavStyle::DireitoGiraShift,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NavStyle::Fenkra => tr("Fenkra"),
            NavStyle::MeioGira => tr("Meio gira, Ctrl+meio move"),
            NavStyle::ShiftMeioGira => tr("Shift+meio gira, meio move"),
            NavStyle::MeioMove => tr("Meio move, meio+direito gira"),
            NavStyle::Blender => tr("Estilo Blender"),
            NavStyle::DireitoGira => tr("Direito gira, Ctrl+direito move"),
            NavStyle::FreeCad => tr("Estilo FreeCAD (CAD)"),
            NavStyle::DireitoGiraShift => tr("Direito gira, Shift+direito move"),
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            NavStyle::Fenkra => tr("Girar: botão esquerdo ou do meio · Mover: direito ou Shift+arrastar · Zoom: roda"),
            NavStyle::MeioGira => tr("Girar: botão do meio · Mover: Ctrl+meio · Zoom: roda (invertida)"),
            NavStyle::ShiftMeioGira => tr("Girar: Shift+meio · Mover: botão do meio · Zoom: roda"),
            NavStyle::MeioMove => tr("Mover: botão do meio · Girar: meio+direito · Zoom: roda"),
            NavStyle::Blender => tr("Girar: botão do meio · Mover: Shift+meio · Zoom: roda ou Ctrl+meio"),
            NavStyle::DireitoGira => tr("Girar: botão direito · Mover: Ctrl+direito ou meio · Zoom: roda"),
            NavStyle::FreeCad => tr("Mover: botão do meio · Girar: meio + esquerdo ou direito · Zoom: roda"),
            NavStyle::DireitoGiraShift => tr("Girar: botão direito · Mover: Shift+direito · Zoom: roda ou Ctrl+direito"),
        }
    }
}

/// Hash FNV-1a 64 do nome em minúsculas. Os nomes antigos de estilos e predefinições, que citavam
/// produtos de terceiros, ficam só como hash: o `config.json` salvo ainda é lido e a regra do projeto
/// (nenhuma marca no código) vale. Ver B202 e B310.
pub(crate) fn hash_nome(n: &str) -> u64 {
    let mut x: u64 = 0xcbf2_9ce4_8422_2325;
    for b in n.to_lowercase().bytes() {
        x = (x ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    x
}

impl NavStyle {
    /// Nome gravado no config.json.
    pub fn nome(self) -> &'static str {
        match self {
            NavStyle::Fenkra => "Fenkra",
            NavStyle::MeioGira => "MeioGira",
            NavStyle::ShiftMeioGira => "ShiftMeioGira",
            NavStyle::MeioMove => "MeioMove",
            NavStyle::Blender => "Blender",
            NavStyle::DireitoGira => "DireitoGira",
            NavStyle::FreeCad => "FreeCad",
            NavStyle::DireitoGiraShift => "DireitoGiraShift",
        }
    }

    /// Lê um nome gravado, novo ou antigo; nome desconhecido cai no padrão, sem erro.
    pub fn de_nome(n: &str) -> NavStyle {
        if let Some(v) = NavStyle::ALL.into_iter().find(|v| v.nome() == n) {
            return v;
        }
        if n == "SolidCadCraft" {
            return NavStyle::Fenkra;
        }
        const LEGADO: [(u64, NavStyle); 6] = [
            (0x8c735c72d6e7e616, NavStyle::MeioGira),
            (0x557ec50bf0c4fc1d, NavStyle::ShiftMeioGira),
            (0x1e2cf6bab7d7815c, NavStyle::ShiftMeioGira),
            (0xc1d4e83b1408f651, NavStyle::MeioMove),
            (0x4f20146f6028e905, NavStyle::DireitoGira),
            (0xb267bd5e49fff919, NavStyle::DireitoGiraShift),
        ];
        let h = hash_nome(n);
        LEGADO.iter().find(|(k, _)| *k == h).map(|(_, v)| *v).unwrap_or_default()
    }
}

impl Serialize for NavStyle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.nome())
    }
}

impl<'de> Deserialize<'de> for NavStyle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(NavStyle::de_nome(&String::deserialize(d)?))
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

/// Qualidade gráfica: finura da malha, anti-serrilhamento e iluminação da viewport.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Graficos {
    /// ajuste de qualidade da malha (peça e montagem)
    pub preset: PresetMalha,
    /// valores da qualidade "Personalizada"
    pub livre: QualidadeMalha,
    pub msaa: Msaa,
    /// durante órbita, pan e zoom desenha sem anti-serrilhamento e volta ao soltar
    pub simplificar_ao_girar: bool,
    /// iluminação de três pontos com ambiente hemisférico e brilho suave
    pub iluminacao_realista: bool,
}

impl Default for Graficos {
    fn default() -> Self {
        Graficos { preset: PresetMalha::Normal, livre: QualidadeMalha::NORMAL, msaa: Msaa::X4, simplificar_ao_girar: false, iluminacao_realista: true }
    }
}

impl Graficos {
    /// Qualidade da malha em uso.
    pub fn qualidade(&self) -> QualidadeMalha {
        self.preset.qualidade(self.livre)
    }

    /// Escolhe um ajuste de qualidade. Na Personalizada parte dos valores do ajuste anterior; a
    /// iluminação realista acompanha: desligada no Rascunho, ligada nos demais.
    pub fn escolher(&mut self, p: PresetMalha) {
        if p == PresetMalha::Personalizada && self.preset != PresetMalha::Personalizada {
            self.livre = self.qualidade();
        }
        self.preset = p;
        match p {
            PresetMalha::Rascunho => self.iluminacao_realista = false,
            PresetMalha::Normal | PresetMalha::Alta => self.iluminacao_realista = true,
            PresetMalha::Personalizada => {}
        }
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
    /// arestas de tangência visíveis (padrão: escondidas, como numa peça real)
    pub tangent_edges: bool,
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
    /// peças, montagens e desenhos abertos/salvos recentemente (arquivos)
    pub recent_files: Vec<String>,
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
    /// famílias de perfis cadastradas pelo usuário (biblioteca estrutural)
    #[serde(default)]
    pub user_profiles: Vec<UserFamily>,
    /// bancada ativa (Modelagem Sólida ou Superfícies)
    #[serde(default)]
    pub bench: Bench,
    /// recálculo automático a cada mudança (desligado: só com "Atualizar")
    pub auto_update: bool,
    /// painel de edição à esquerda (junto da árvore) em vez da direita
    pub editor_left: bool,
    /// sketch novo num plano (não numa face do corpo) vai para a pasta de geometria
    pub sketch_in_geometry: bool,
    /// símbolo dos pontos de referência: 0 cruz 3D, 1 asterisco, 2 X, 3 círculo, 4 quadrado
    pub point_style: u8,
    /// tamanho do símbolo do ponto (px)
    pub point_size: f32,
    /// espessura (px) e tipo (0 contínua, 1 tracejada, 2 traço-ponto, 3 pontilhada) das linhas e curvas de referência
    pub ref_line_width: f32,
    pub ref_line_dash: u8,
    /// marca dos centros de furo no sketch: 0 asterisco, 1 cruz, 2 círculo com cruz
    pub hole_mark: u8,
    /// seções da árvore ocultadas pelo usuário (chaves de `TREE_SECTIONS`)
    pub tree_hidden: Vec<String>,
    /// seleção em caixa: 0 com Ctrl + arrastar, 1 janela, 2 cruzada, 3 pela direção (arrastar sem Ctrl)
    pub box_mode: u8,
    /// cores salvas pelo usuário (nome, cor)
    pub user_colors: Vec<(String, [u8; 3])>,
    /// tema dos menus: 0 padrão, 1 grafite (preto/cinza), 2 carmim (preto/vermelho), 3 azul
    pub menu_theme: u8,
    /// tema da interface (pronto ou do usuário)
    pub theme_name: String,
    /// estilo geral das anotações 3D (o que cada anotação deixa em "padrão")
    pub pmi_style: fenkra_core::pmi::PmiStyle,
    /// furos flangeados cadastrados pelo usuário (nome, diâmetro, altura do colar)
    pub flanged_holes: Vec<(String, f64, f64)>,
    /// última macro escrita na janela de automação
    pub macro_src: String,
    /// qualidade gráfica (malha, anti-serrilhamento, iluminação)
    pub graficos: Graficos,
    /// posição dos painéis livres, por bancada
    pub paineis: Vec<crate::paineis::PainelSalvo>,
    /// grupos fora do contexto que o usuário deixou sempre na faixa
    pub barra_fixos: Vec<Group>,
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
            theme: Theme::Claro,
            ui_scale: 1.0,
            gradient: true,
            bg_top: crate::theme::default_def().bg_top,
            bg_bottom: crate::theme::default_def().bg_bottom,
            display: DisplayMode::SombreadoArestas,
            edges_in_render: false,
            tangent_edges: false,
            ortho: false,
            filter: Filter::default(),
            docks: Group::ALL.iter().map(|g| (*g, Dock::Topo)).collect(),
            families: vec![],
            nav: NavStyle::Fenkra,
            invert_zoom: false,
            sel_faces: true,
            sel_edges: true,
            sel_vertices: true,
            recent: vec![],
            recent_files: vec![],
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
            user_profiles: vec![],
            macro_src: String::new(),
            bench: Default::default(),
            auto_update: true,
            editor_left: true,
            sketch_in_geometry: true,
            point_style: 1,
            point_size: 7.,
            ref_line_width: 1.5,
            ref_line_dash: 0,
            hole_mark: 0,
            tree_hidden: vec![],
            box_mode: 0,
            user_colors: vec![],
            menu_theme: 4,
            theme_name: crate::theme::PADRAO.into(),
            pmi_style: Default::default(),
            flanged_holes: vec![],
            graficos: Default::default(),
            paineis: vec![],
            barra_fixos: vec![],
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
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("config.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default()
            .migrar_tema()
    }

    /// Quem tinha um tema que não existe mais (removido) cai no padrão, com o fundo da viewport dele.
    pub fn migrar_tema(mut self) -> Self {
        crate::theme::reload_user();
        if crate::theme::find(&self.theme_name).is_none() {
            let d = crate::theme::default_def();
            self.theme_name = d.name.clone();
            self.theme = if d.dark { Theme::Escuro } else { Theme::Claro };
            self.bg_top = d.bg_top;
            self.bg_bottom = d.bg_bottom;
        }
        self
    }

    pub fn save(&self) {
        // execuções de demonstração/teste (variáveis FENKRA_DEMO_*, FENKRA_ABRIR) não mexem nas preferências
        if std::env::vars().any(|(k, _)| k.starts_with("FENKRA_DEMO") || k == "FENKRA_ABRIR" || k == "FENKRA_BANCADA") {
            return;
        }
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

    pub fn add_recent_file(&mut self, path: &std::path::Path) {
        let p = path.to_string_lossy().to_string();
        self.recent_files.retain(|x| *x != p);
        self.recent_files.insert(0, p);
        self.recent_files.truncate(12);
        self.save();
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
            msaa: self.graficos.msaa.amostras(),
            simplificar: self.graficos.simplificar_ao_girar,
            navegando: false,
            realista: self.graficos.iluminacao_realista,
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let def = crate::theme::find(&self.theme_name).unwrap_or_else(crate::theme::default_def);
        crate::theme::apply(ctx, &def);
        ctx.set_zoom_factor(self.ui_scale.clamp(0.7, 2.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuracao_antiga_sem_graficos_usa_os_padroes() {
        let s: Settings = serde_json::from_str(r#"{"ui_scale": 1.2, "grid": false}"#).unwrap();
        assert_eq!(s.graficos, Graficos::default());
        assert_eq!(s.graficos.qualidade(), QualidadeMalha::NORMAL);
        assert_eq!(s.graficos.msaa, Msaa::X4);
        assert!(s.graficos.iluminacao_realista && !s.graficos.simplificar_ao_girar);
        // campos soltos da seção também têm padrão
        let g: Settings = serde_json::from_str(r#"{"graficos": {"preset": "Alta"}}"#).unwrap();
        assert_eq!(g.graficos.qualidade(), QualidadeMalha::ALTA);
        assert_eq!(g.graficos.msaa, Msaa::X4);
    }

    /// Nomes antigos (montados em pedaços para o código não citar marca) migram para o equivalente.
    #[test]
    fn config_com_estilo_e_predefinicao_antigos_migra_sem_erro() {
        let nome = |a: &str, b: &str| [a, b].concat();
        let casos = [
            (nome("Solid", "Works"), NavStyle::MeioGira, crate::keymap::Preset::VistasCtrl),
            (nome("Cat", "ia"), NavStyle::MeioMove, crate::keymap::Preset::PoucosAtalhos),
            (nome("Fus", "ion"), NavStyle::ShiftMeioGira, crate::keymap::Preset::LetraF6),
            (nome("Onsh", "ape"), NavStyle::DireitoGira, crate::keymap::Preset::VistasShift),
            (nome("Inven", "tor"), NavStyle::ShiftMeioGira, crate::keymap::Preset::LetraHome),
        ];
        for (antigo, nav, preset) in casos {
            let j = format!(r#"{{"nav": "{antigo}", "keys": {{"preset": "{antigo}"}}}}"#);
            let s: Settings = serde_json::from_str(&j).unwrap();
            assert_eq!(s.nav, nav, "{antigo}");
            assert_eq!(s.keys.preset, preset, "{antigo}");
        }
        let rhino: Settings = serde_json::from_str(&format!(r#"{{"nav": "{}"}}"#, nome("Rhi", "no"))).unwrap();
        assert_eq!(rhino.nav, NavStyle::DireitoGiraShift);
        // nome desconhecido não derruba a leitura
        let x: Settings = serde_json::from_str(r#"{"nav": "Qualquer", "keys": {"preset": "Outro"}}"#).unwrap();
        assert_eq!((x.nav, x.keys.preset), (NavStyle::Fenkra, crate::keymap::Preset::Fenkra));
    }

    #[test]
    fn nomes_novos_de_estilo_e_predefinicao_vao_e_voltam() {
        for n in NavStyle::ALL {
            let j = serde_json::to_string(&n).unwrap();
            assert_eq!(serde_json::from_str::<NavStyle>(&j).unwrap(), n);
        }
        for p in crate::keymap::Preset::ALL {
            let j = serde_json::to_string(&p).unwrap();
            assert_eq!(serde_json::from_str::<crate::keymap::Preset>(&j).unwrap(), p);
        }
    }

    #[test]
    fn graficos_vao_e_voltam_do_arquivo() {
        let mut s = Settings::default();
        s.graficos.escolher(PresetMalha::Personalizada);
        s.graficos.livre = QualidadeMalha { desvio_rel: 0.001, angulo_graus: 15. };
        s.graficos.msaa = Msaa::X8;
        s.graficos.simplificar_ao_girar = true;
        let volta: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(volta.graficos, s.graficos);
        assert_eq!(volta.graficos.qualidade(), QualidadeMalha { desvio_rel: 0.001, angulo_graus: 15. });
    }

    #[test]
    fn escolher_preset_ajusta_a_iluminacao_e_parte_do_anterior() {
        let mut g = Graficos::default();
        g.escolher(PresetMalha::Rascunho);
        assert!(!g.iluminacao_realista);
        g.escolher(PresetMalha::Alta);
        assert!(g.iluminacao_realista);
        g.escolher(PresetMalha::Personalizada);
        assert_eq!(g.qualidade(), QualidadeMalha::ALTA, "a Personalizada parte dos valores do ajuste anterior");
    }
}
