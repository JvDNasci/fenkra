// SPDX-License-Identifier: LGPL-2.1-or-later
//! Temas da interface: cores dos painéis, janelas, texto, destaque, faixas da barra e fundo da
//! viewport. Oito temas prontos e os do usuário (arquivos .json em
//! %APPDATA%\Fenkra\temas — uma extensão é só um arquivo desses).

use eframe::egui::{self, Color32, Stroke};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use crate::acessibilidade::{cor_estado, cor_papel, paleta_segura, Estado, Papel};

/// Contorno dos painéis e janelas: fino ou nenhum.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Contorno {
    #[default]
    Fino,
    Nenhum,
}

/// Conjunto de fontes do tema. `Padrao` mantém a fonte atual da interface.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Fonte {
    #[default]
    Padrao,
    /// IBM Plex Sans (texto e títulos) e IBM Plex Mono (valores)
    Prancheta,
    /// Barlow (texto), Barlow Condensed (títulos) e JetBrains Mono (valores)
    Cianotipia,
}

fn d_canto() -> u8 {
    6
}
fn d_densidade() -> f32 {
    1.0
}
fn d_sombra() -> bool {
    true
}

/// Cor da face da peça quando o corpo usa a cor padrão do documento.
pub const PECA_PADRAO: [u8; 3] = [178, 180, 226];

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ThemeDef {
    pub name: String,
    pub dark: bool,
    /// fundo dos painéis, das janelas e dos campos
    pub panel: [u8; 3],
    pub window: [u8; 3],
    pub field: [u8; 3],
    pub text: [u8; 3],
    /// cor de destaque (seleção, botões ativos, links)
    pub accent: [u8; 3],
    /// faixas alternadas dos blocos da barra de comandos
    pub band_a: [u8; 3],
    pub band_b: [u8; 3],
    /// fundo da viewport (degradê)
    pub bg_top: [u8; 3],
    pub bg_bottom: [u8; 3],
    // ---- estilo completo; temas antigos em JSON não têm estes campos e caem nos padrões ----
    /// raio dos cantos dos botões e campos (0 = reto); janelas e menus derivam dele
    #[serde(default = "d_canto")]
    pub canto: u8,
    #[serde(default)]
    pub contorno: Contorno,
    /// fator dos espaçamentos (1 = padrão; menor = mais compacto)
    #[serde(default = "d_densidade")]
    pub densidade: f32,
    /// sombra sob janelas e menus
    #[serde(default = "d_sombra")]
    pub sombra: bool,
    /// cor das linhas de separação e contornos (padrão: mistura de painel e texto)
    #[serde(default)]
    pub linha: Option<[u8; 3]>,
    /// cor da face da peça na viewport (padrão: a do documento)
    #[serde(default)]
    pub peca: Option<[u8; 3]>,
    /// cor das arestas da peça
    #[serde(default)]
    pub aresta: Option<[u8; 3]>,
    /// cor de links e destaques de texto (padrão: o destaque)
    #[serde(default)]
    pub texto_destaque: Option<[u8; 3]>,
    /// ícones: cor da base do desenho e cor do resultado (padrão: texto e destaque)
    #[serde(default)]
    pub icone_base: Option<[u8; 3]>,
    #[serde(default)]
    pub icone_resultado: Option<[u8; 3]>,
    /// conjunto de fontes (corpo, títulos e valores)
    #[serde(default)]
    pub fonte: Fonte,
    /// fios de arabesco nos separadores da tela inicial e nos estados vazios (B314)
    #[serde(default)]
    pub arabesco: bool,
}

fn hex(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

fn mixa(a: [u8; 3], b: [u8; 3], k: f32) -> [u8; 3] {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
    [m(0), m(1), m(2)]
}

/// Monta um tema pronto a partir das cores-chave; o resto (janela, campo, faixas, degradê) deriva delas.
#[allow(clippy::too_many_arguments)]
fn tema(name: &str, dark: bool, fundo: u32, painel: u32, texto: u32, destaque: u32, linha: u32, peca: u32, canto: u8, sombra: bool, densidade: f32) -> ThemeDef {
    let (fundo, painel, texto, destaque, linha, peca) = (hex(fundo), hex(painel), hex(texto), hex(destaque), hex(linha), hex(peca));
    let (branco, preto) = ([255; 3], [0; 3]);
    let (window, field) = if dark { (mixa(painel, texto, 0.05), mixa(painel, preto, 0.35)) } else { (mixa(painel, branco, 0.45), mixa(painel, branco, 0.8)) };
    let (bg_top, bg_bottom) = if dark { (mixa(fundo, branco, 0.06), mixa(fundo, preto, 0.25)) } else { (mixa(fundo, branco, 0.5), fundo) };
    ThemeDef {
        name: name.into(),
        dark,
        panel: painel,
        window,
        field,
        text: texto,
        accent: destaque,
        band_a: painel,
        band_b: mixa(painel, texto, 0.07),
        bg_top,
        bg_bottom,
        canto,
        contorno: Contorno::Fino,
        densidade,
        sombra,
        linha: Some(linha),
        peca: Some(peca),
        aresta: Some(mixa(peca, preto, 0.8)),
        texto_destaque: Some(destaque),
        icone_base: Some(texto),
        icone_resultado: Some(destaque),
        fonte: if dark && canto > 0 { Fonte::Cianotipia } else if canto == 0 { Fonte::Prancheta } else { Fonte::Padrao },
        arabesco: false,
    }
}

/// Nome do tema Fênix, o único que liga o arabesco.
pub const FENIX: &str = "Fênix";

/// O tema em uso liga o arabesco?
pub fn arabesco_ativo() -> bool {
    current().is_some_and(|d| d.arabesco)
}

/// Nome do tema padrão (e do que recebe quem tinha um tema removido).
pub const PADRAO: &str = "Prancheta Fria";

/// Temas prontos: duas opções neutras e sete com estilo próprio (cantos, contornos, densidade, sombra e peça).
pub fn builtin() -> Vec<ThemeDef> {
    let neutro = |name: &str, dark: bool, panel: [u8; 3], window: [u8; 3], field: [u8; 3], text: [u8; 3], accent: [u8; 3], band_a: [u8; 3], band_b: [u8; 3], bg_top: [u8; 3], bg_bottom: [u8; 3]| ThemeDef {
        name: name.into(),
        dark,
        panel,
        window,
        field,
        text,
        accent,
        band_a,
        band_b,
        bg_top,
        bg_bottom,
        canto: 6,
        contorno: Contorno::Fino,
        densidade: 1.0,
        sombra: true,
        linha: None,
        peca: None,
        aresta: None,
        texto_destaque: None,
        icone_base: None,
        icone_resultado: None,
        fonte: Fonte::Padrao,
        arabesco: false,
    };
    vec![
        // fundo da viewport, painel, texto, destaque, linha, peça, canto, sombra, densidade
        // texto e destaque são escuros e, em 24 px, base e resultado do ícone viravam uma cor só:
        // a base clareia para o resultado se destacar pelo tom
        ThemeDef { icone_base: Some(hex(0x7F8E97)), ..tema("Prancheta Fria", false, 0xE5EBEE, 0xE5EBEE, 0x16242E, 0x0B6F8E, 0xAEBDC5, 0xA9B8C1, 0, false, 0.92) },
        tema("Prancheta Papel", false, 0xEEEEE9, 0xEEEEE9, 0x1B1C1E, 0xC23A1E, 0xB9B9B2, 0xB4BCC4, 0, false, 0.92),
        tema("Prancheta Noturna", true, 0x23272B, 0x23272B, 0xE4E3DC, 0xF08A5D, 0x4A5056, 0x7C8791, 0, false, 0.92),
        tema("Cianotipia Azul", true, 0x14405F, 0x0F324C, 0xEAF2F8, 0xFFD166, 0x2C6088, 0x7E97AB, 2, true, 1.0),
        tema("Cianotipia Petróleo", true, 0x10474A, 0x0B3538, 0xE9F5F1, 0xFFB86B, 0x2B6F70, 0x7FA39C, 2, true, 1.0),
        tema("Cianotipia Noturna", true, 0x1A2330, 0x131A24, 0xDDE5EE, 0x7FD1E8, 0x3A4A5E, 0x8593A4, 2, true, 1.0),
        tema("Cianotipia Sépia", true, 0x3B2A22, 0x2C1F19, 0xF3E9DC, 0xEDB26F, 0x6B5245, 0xB39C84, 2, true, 1.0),
        // B314: fundo calmo de papel quente, texto grafite e um só acento (brasa); o arabesco fica
        // nos separadores e nos estados vazios, nunca na área de trabalho
        ThemeDef { arabesco: true, icone_base: Some(hex(0x7A6E66)), ..tema(FENIX, false, 0xE9E3DB, 0xEFEAE3, 0x2A2321, 0xB4461E, 0xCDC3B8, 0xA39384, 4, false, 1.0) },
        neutro("Escuro", true, [27, 29, 33], [32, 34, 39], [14, 15, 18], [225, 228, 234], [70, 140, 230], [30, 32, 37], [40, 43, 50], [74, 84, 100], [26, 29, 34]),
        ThemeDef { peca: Some([118, 122, 170]), ..neutro("Claro", false, [238, 240, 243], [248, 249, 251], [255, 255, 255], [24, 28, 34], [30, 110, 220], [228, 231, 236], [240, 242, 246], [214, 222, 232], [158, 170, 186]) },
    ]
}

/// O tema padrão.
pub fn default_def() -> ThemeDef {
    builtin().into_iter().find(|t| t.name == PADRAO).expect("tema padrão existe")
}

/// Pasta dos temas do usuário.
pub fn user_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Fenkra").join("temas"))
}

static USER: RwLock<Vec<ThemeDef>> = RwLock::new(Vec::new());
static CURRENT: RwLock<Option<ThemeDef>> = RwLock::new(None);

/// Relê os temas do usuário (cada .json da pasta).
pub fn reload_user() -> usize {
    let mut v = vec![];
    if let Some(d) = user_dir() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                if e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("json")) {
                    if let Some(t) = std::fs::read_to_string(e.path()).ok().and_then(|s| serde_json::from_str::<ThemeDef>(&s).ok()) {
                        v.push(t);
                    }
                }
            }
        }
    }
    let n = v.len();
    if let Ok(mut g) = USER.write() {
        *g = v;
    }
    n
}

/// Todos os temas (prontos + do usuário).
pub fn all() -> Vec<ThemeDef> {
    let mut v = builtin();
    if let Ok(g) = USER.read() {
        v.extend(g.iter().cloned());
    }
    v
}

pub fn find(name: &str) -> Option<ThemeDef> {
    all().into_iter().find(|t| t.name == name)
}

/// Tema em uso (para as faixas da barra e outras pinturas).
pub fn current() -> Option<ThemeDef> {
    CURRENT.read().ok().and_then(|g| g.clone())
}

/// Grava o tema atual como modelo editável na pasta do usuário; devolve o caminho.
pub fn export_template(def: &ThemeDef) -> Result<PathBuf, String> {
    let d = user_dir().ok_or("pasta de configuração não encontrada")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let mut t = def.clone();
    t.name = format!("{} (meu)", def.name);
    let p = d.join(format!("{}.json", t.name.replace(['/', '\\', ':'], "_")));
    std::fs::write(&p, serde_json::to_string_pretty(&t).unwrap_or_default()).map_err(|e| e.to_string())?;
    reload_user();
    Ok(p)
}

static BASE_FONTS: RwLock<Option<egui::FontDefinitions>> = RwLock::new(None);
static FONTE_ATUAL: RwLock<Option<Fonte>> = RwLock::new(None);
static REV: AtomicU64 = AtomicU64::new(1);

/// Guarda as fontes do sistema (com símbolos) que servem de reserva sob as fontes do tema.
/// Chamada uma vez na partida; as fontes só são carregadas no `apply`, ao trocar de tema.
pub fn set_base_fonts(f: egui::FontDefinitions) {
    if let Ok(mut g) = BASE_FONTS.write() {
        *g = Some(f);
    }
    if let Ok(mut g) = FONTE_ATUAL.write() {
        *g = None;
    }
}

/// Número que muda a cada troca de tema (para quem guarda cache de cor, como os ícones).
pub fn theme_rev() -> u64 {
    REV.load(Ordering::Relaxed)
}

/// Cores dos ícones do tema em uso: (base, resultado).
pub fn icon_colors() -> ([u8; 3], [u8; 3]) {
    match current() {
        Some(d) => (d.icone_base.unwrap_or(d.text), d.icone_resultado.unwrap_or(d.accent)),
        None => ([225, 228, 234], [70, 140, 230]),
    }
}

/// Cor da face da peça e das arestas definidas pelo tema (None = usa a do documento / padrão da cena).
pub fn piece_colors() -> (Option<[u8; 3]>, Option<[u8; 3]>) {
    current().map(|d| (d.peca, d.aresta)).unwrap_or((None, None))
}

fn font_data(bytes: &'static [u8]) -> std::sync::Arc<egui::FontData> {
    std::sync::Arc::new(egui::FontData::from_static(bytes))
}

/// Monta as definições de fonte do conjunto, com as de reserva (símbolos, ícones) por baixo.
fn fonts_for(f: Fonte, base: &egui::FontDefinitions) -> egui::FontDefinitions {
    let mut fd = base.clone();
    let lista: &[(&str, &'static [u8])] = match f {
        Fonte::Padrao => return fd,
        Fonte::Prancheta => &[
            ("tema_corpo", include_bytes!("../../../assets/fonts/IBMPlexSans-Regular.ttf")),
            ("tema_titulo", include_bytes!("../../../assets/fonts/IBMPlexSans-SemiBold.ttf")),
            ("tema_valor", include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf")),
        ],
        Fonte::Cianotipia => &[
            ("tema_corpo", include_bytes!("../../../assets/fonts/Barlow-Regular.ttf")),
            ("tema_titulo", include_bytes!("../../../assets/fonts/BarlowCondensed-SemiBold.ttf")),
            ("tema_valor", include_bytes!("../../../assets/fonts/JetBrainsMono-Medium.ttf")),
        ],
    };
    for (k, b) in lista {
        fd.font_data.insert((*k).into(), font_data(b));
    }
    let reserva_prop = fd.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    let reserva_mono = fd.families.get(&egui::FontFamily::Monospace).cloned().unwrap_or_default();
    let com = |first: &str, rest: &[String]| -> Vec<String> {
        let mut v = vec![first.to_string()];
        v.extend(rest.iter().cloned());
        v
    };
    fd.families.insert(egui::FontFamily::Proportional, com("tema_corpo", &reserva_prop));
    fd.families.insert(egui::FontFamily::Monospace, com("tema_valor", &reserva_mono));
    fd.families.insert(egui::FontFamily::Name(TITULO.into()), com("tema_titulo", &reserva_prop));
    fd
}

/// Família dos títulos (cai na proporcional quando o tema não define uma).
const TITULO: &str = "tema_titulo";

fn apply_fonts(ctx: &egui::Context, f: Fonte) {
    if FONTE_ATUAL.read().ok().is_some_and(|g| *g == Some(f)) {
        return;
    }
    let Some(base) = BASE_FONTS.read().ok().and_then(|g| g.clone()) else { return };
    ctx.set_fonts(fonts_for(f, &base));
    if let Ok(mut g) = FONTE_ATUAL.write() {
        *g = Some(f);
    }
}

fn c(x: [u8; 3]) -> Color32 {
    Color32::from_rgb(x[0], x[1], x[2])
}

fn mix(a: [u8; 3], b: [u8; 3], k: f32) -> Color32 {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
    Color32::from_rgb(m(0), m(1), m(2))
}

/// Aplica o tema na interface: cores, cantos, contornos, densidade, sombra e fontes.
/// As fontes só são trocadas quando o conjunto muda (nunca por quadro).
pub fn apply(ctx: &egui::Context, def: &ThemeDef) {
    // o egui guarda um estilo para o tema claro e outro para o escuro e escolhe pelo do sistema:
    // sem fixar a preferência, o tema escolhido podia cair no estilo que a tela não usa (B335)
    ctx.set_theme(if def.dark { egui::ThemePreference::Dark } else { egui::ThemePreference::Light });
    let mut v = if def.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let linha = def.linha.unwrap_or_else(|| mixa(def.panel, def.text, 0.16));
    let linha_fraca = if def.linha.is_some() { linha } else { mixa(def.panel, def.text, 0.12) };
    let fino = def.contorno == Contorno::Fino;
    let canto = def.canto.min(16);
    let (r_widget, r_janela, r_menu) = if canto == 0 { (0, 0, 0) } else { (canto, canto + 4, canto + 2) };
    v.panel_fill = c(def.panel);
    v.window_fill = c(def.window);
    v.extreme_bg_color = c(def.field);
    v.faint_bg_color = mix(def.panel, def.text, 0.05);
    v.code_bg_color = mix(def.panel, def.text, 0.08);
    v.override_text_color = Some(c(def.text));
    v.hyperlink_color = c(def.texto_destaque.unwrap_or(def.accent));
    v.selection.bg_fill = mix(def.panel, def.accent, if def.dark { 0.55 } else { 0.35 });
    // foco do teclado: o egui o desenha com este traço; 2 px cumprem os 3:1 de componente (B321)
    v.selection.stroke = Stroke::new(2., c(def.accent));
    v.window_stroke = if fino { Stroke::new(1., c(linha)) } else { Stroke::NONE };
    v.window_corner_radius = egui::CornerRadius::same(r_janela);
    v.menu_corner_radius = egui::CornerRadius::same(r_menu);
    if !def.sombra {
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
    }
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c(def.panel);
    w.noninteractive.weak_bg_fill = c(def.panel);
    w.noninteractive.bg_stroke = if fino { Stroke::new(1., c(linha_fraca)) } else { Stroke::NONE };
    w.noninteractive.fg_stroke = Stroke::new(1., mix(def.panel, def.text, 0.85));
    // campos e caixas de marcar sempre visíveis: preenchimento e contorno com contraste
    w.inactive.bg_fill = mix(def.panel, def.text, if def.dark { 0.16 } else { 0.10 });
    w.inactive.weak_bg_fill = mix(def.panel, def.text, 0.07);
    w.inactive.bg_stroke = Stroke::new(1., if def.linha.is_some() { mix(linha, def.text, 0.25) } else { mix(def.panel, def.text, if def.dark { 0.30 } else { 0.28 }) });
    w.inactive.fg_stroke = Stroke::new(1., c(def.text));
    w.hovered.bg_fill = mix(def.panel, def.accent, 0.28);
    w.hovered.weak_bg_fill = mix(def.panel, def.accent, 0.22);
    w.hovered.bg_stroke = Stroke::new(1., mix(def.panel, def.accent, 0.7));
    w.hovered.fg_stroke = Stroke::new(1.5, c(def.text));
    w.active.bg_fill = mix(def.panel, def.accent, 0.45);
    w.active.weak_bg_fill = mix(def.panel, def.accent, 0.4);
    w.active.bg_stroke = Stroke::new(1., c(def.accent));
    w.open.bg_fill = mix(def.panel, def.accent, 0.2);
    w.open.weak_bg_fill = mix(def.panel, def.accent, 0.16);
    for x in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        x.corner_radius = egui::CornerRadius::same(r_widget);
    }
    ctx.set_visuals(v);
    // densidade: escala os espaçamentos a partir dos padrões do egui (não acumula entre trocas)
    let k = def.densidade.clamp(0.7, 1.4);
    let base = egui::Style::default();
    let titulo = if def.fonte == Fonte::Padrao { egui::FontFamily::Proportional } else { egui::FontFamily::Name(TITULO.into()) };
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = base.spacing.item_spacing * k;
        s.spacing.button_padding = base.spacing.button_padding * k;
        s.spacing.window_margin = egui::Margin::same((base.spacing.window_margin.left as f32 * k).round() as i8);
        s.spacing.menu_margin = egui::Margin::same((base.spacing.menu_margin.left as f32 * k).round() as i8);
        if let Some(h) = s.text_styles.get_mut(&egui::TextStyle::Heading) {
            h.family = titulo;
        }
    });
    apply_fonts(ctx, def.fonte);
    let mudou = current().is_none_or(|a| a != *def);
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(def.clone());
    }
    if mudou {
        REV.fetch_add(1, Ordering::Relaxed);
    }
}

/// Cor de destaque cheia do tema (botões ativos, ícones de cabeçalho); sem tema, a da seleção.
pub fn accent(v: &egui::Visuals) -> Color32 {
    current().map(|d| c(d.accent)).unwrap_or(v.selection.stroke.color)
}

/// Cor de status legível no fundo do painel: no claro, escurece (verde/amarelo claros somem).
pub fn readable(col: Color32, v: &egui::Visuals) -> Color32 {
    if v.dark_mode { col } else { col.lerp_to_gamma(Color32::BLACK, 0.45) }
}

/// Faixas da barra (fundo A, fundo B, destaque) do tema em uso.
pub fn bands() -> Option<(Color32, Color32, Color32)> {
    current().map(|d| (c(d.band_a), c(d.band_b), c(d.accent)))
}

/// Miniatura do tema (para escolher): faixa da barra, painel, destaque e viewport.
pub fn swatch(ui: &mut egui::Ui, def: &ThemeDef, selected: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(132., 76.), egui::Sense::click());
    // B321: anel de foco quando o Tab chega aqui
    let resp = crate::teclado::anel(ui, resp);
    let p = ui.painter();
    let hover = resp.hovered();
    p.rect_filled(r, 8., c(def.panel));
    // viewport em degradê
    let vp = egui::Rect::from_min_max(r.min + egui::vec2(34., 18.), r.max - egui::vec2(6., 18.));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(vp.left_top(), c(def.bg_top));
    mesh.colored_vertex(vp.right_top(), c(def.bg_top));
    mesh.colored_vertex(vp.right_bottom(), c(def.bg_bottom));
    mesh.colored_vertex(vp.left_bottom(), c(def.bg_bottom));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    // barra de comandos e árvore
    p.rect_filled(egui::Rect::from_min_size(r.min + egui::vec2(4., 4.), egui::vec2(r.width() - 8., 10.)), 3., c(def.band_b));
    for i in 0..5 {
        p.rect_filled(egui::Rect::from_min_size(r.min + egui::vec2(8. + i as f32 * 12., 6.), egui::vec2(7., 6.)), 1.5, c(def.accent));
    }
    for i in 0..4 {
        p.rect_filled(egui::Rect::from_min_size(r.min + egui::vec2(6., 22. + i as f32 * 9.), egui::vec2(22., 4.)), 1., mix(def.panel, def.text, 0.4));
    }
    // peça na viewport
    let cc = vp.center();
    p.circle_filled(cc, 9., c(def.peca.unwrap_or(PECA_PADRAO)));
    p.circle_stroke(cc, 9., Stroke::new(1., c(def.aresta.unwrap_or([20, 22, 26]))));
    p.text(egui::pos2(r.center().x, r.bottom() - 9.), egui::Align2::CENTER_CENTER, crate::i18n::tr(&def.name), egui::FontId::proportional(10.5), c(def.text));
    let stroke = if selected { Stroke::new(2.5, c(def.accent)) } else if hover { Stroke::new(1.5, mix(def.panel, def.accent, 0.6)) } else { Stroke::new(1., mix(def.panel, def.text, 0.2)) };
    p.rect_stroke(r, 8., stroke, egui::StrokeKind::Inside);
    resp
}

fn dark_now() -> bool {
    current().is_none_or(|d| d.dark)
}

/// Cores de estado legíveis no tema em uso (ok, aviso, erro). Com a paleta segura (B321) saem do
/// par azul x laranja, que as três deficiências de cor separam; o símbolo do estado acompanha a cor.
/// Cor calculada numa escala de análise, a partir de paradas do tema.
pub fn cor_da_escala(c: [f32; 3]) -> Color32 {
    let [r, g, b] = c.map(|x| (x.clamp(0., 1.) * 255.) as u8);
    Color32::from_rgb(r, g, b)
}

/// Cor opaca a partir dos três canais guardados no tema.
pub fn cor_rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A mesma cor do tema com outra opacidade (0 a 255), sem criar cor nova.
pub fn com_alfa(c: Color32, alfa: u8) -> Color32 {
    let [r, g, b, _] = c.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, alfa)
}

pub fn st_ok() -> Color32 {
    estado(Estado::Ok)
}
pub fn st_warn() -> Color32 {
    estado(Estado::Aviso)
}
pub fn st_err() -> Color32 {
    estado(Estado::Erro)
}
fn estado(e: Estado) -> Color32 {
    c(cor_estado(e, dark_now(), paleta_segura()))
}

/// Cores por papel na seleção do comando e na prévia (B246, B247). Ficam longe do azul da peça, do
/// aviso e do erro, para o volume que sai, a ferramenta e o plano neutro nunca se confundirem.
pub fn papel_retirada() -> Color32 {
    c(cor_papel(Papel::Retirada, dark_now(), paleta_segura()))
}
pub fn papel_ferramenta() -> Color32 {
    c(cor_papel(Papel::Ferramenta, dark_now(), paleta_segura()))
}
pub fn papel_neutro() -> Color32 {
    c(cor_papel(Papel::Neutro, dark_now(), paleta_segura()))
}
/// Corpo alvo da booleana, da divisão e do mover (B300).
pub fn papel_alvo() -> Color32 {
    c(cor_papel(Papel::Alvo, dark_now(), paleta_segura()))
}

/// Cor que vem de um dado da pessoa (uma regra, a aparência de um furo), não do tema.
pub fn cor_do_dado(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: [u8; 3]) -> f64 {
        let f = |v: u8| {
            let s = v as f64 / 255.;
            if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
    }
    fn contraste(a: [u8; 3], b: [u8; 3]) -> f64 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn temas_prontos_tem_contraste_e_peca_distinta() {
        let b = builtin();
        assert_eq!(b.len(), 10);
        for t in &b {
            for (nome, fundo) in [("painel", t.panel), ("janela", t.window), ("campo", t.field)] {
                let r = contraste(t.text, fundo);
                assert!(r >= 4.5, "{}: texto sobre {nome} tem contraste {r:.2}", t.name);
            }
            let peca = t.peca.unwrap_or(PECA_PADRAO);
            for (nome, fundo) in [("topo", t.bg_top), ("base", t.bg_bottom)] {
                let r = contraste(peca, fundo);
                assert!(peca != fundo && r >= 1.25, "{}: peça confunde com o fundo ({nome}) {r:.2}", t.name);
            }
        }
    }

    #[test]
    fn contraste_minimo_vale_nas_tres_densidades() {
        for base in builtin() {
            for dens in [0.85_f32, 1.0, 1.15] {
                let mut t = base.clone();
                t.densidade = dens;
                for (nome, fundo) in [("painel", t.panel), ("janela", t.window), ("campo", t.field)] {
                    let r = contraste(t.text, fundo);
                    assert!(r >= 4.5, "{} (densidade {dens}): texto sobre {nome} tem contraste {r:.2}", t.name);
                }
            }
        }
    }

    #[test]
    fn prancheta_fria_separa_base_e_resultado_dos_icones() {
        let t = find("Prancheta Fria").expect("tema");
        let base = t.icone_base.expect("base");
        let res = t.icone_resultado.expect("resultado");
        assert!(lum(base) > lum(res) * 1.5, "base perto demais do resultado");
        assert!(contraste(base, t.panel) >= 2.2, "base some no fundo");
    }

    #[test]
    fn nomes_sem_marca_e_padrao_existe() {
        let proibidos = ["neotech", "fusion", "titanium", "aerovision", "cybercad"];
        for t in builtin() {
            let n = t.name.to_lowercase();
            assert!(!proibidos.iter().any(|p| n.contains(p)), "{}", t.name);
        }
        assert_eq!(default_def().name, PADRAO);
    }

    #[test]
    fn json_antigo_de_tema_carrega_com_padroes() {
        let j = r#"{"name":"Meu","dark":true,"panel":[1,2,3],"window":[4,5,6],"field":[7,8,9],"text":[250,250,250],"accent":[1,1,1],"band_a":[0,0,0],"band_b":[0,0,0],"bg_top":[0,0,0],"bg_bottom":[0,0,0]}"#;
        let t: ThemeDef = serde_json::from_str(j).unwrap();
        assert_eq!((t.canto, t.contorno, t.sombra, t.fonte), (6, Contorno::Fino, true, Fonte::Padrao));
        assert!((t.densidade - 1.0).abs() < 1e-6 && t.peca.is_none() && t.icone_base.is_none());
    }

    #[test]
    fn trocar_o_tema_muda_a_revisao_e_as_cores_dos_icones() {
        let ctx = egui::Context::default();
        let b = builtin();
        apply(&ctx, &b[0]);
        let (r0, i0) = (theme_rev(), icon_colors());
        apply(&ctx, &b[0]);
        assert_eq!(theme_rev(), r0, "reaplicar o mesmo tema não muda a revisão");
        apply(&ctx, &b[3]);
        assert_ne!(theme_rev(), r0);
        assert_ne!(icon_colors(), i0);
        assert_eq!(ctx.global_style().visuals.widgets.inactive.corner_radius, egui::CornerRadius::same(2));
    }

    #[test]
    fn fontes_dos_temas_cobrem_acentos_e_simbolos_tecnicos() {
        let base = egui::FontDefinitions::empty();
        for f in [Fonte::Prancheta, Fonte::Cianotipia] {
            let fd = fonts_for(f, &base);
            let ctx = egui::Context::default();
            ctx.set_fonts(fd);
            let mut saida = ctx.run_ui(Default::default(), |_| {});
            saida.textures_delta.clear();
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace, egui::FontFamily::Name(TITULO.into())] {
                let id = egui::FontId::new(14., fam.clone());
                for ch in "ØØ±°áâãçéêíóõúÁÇÃÕ".chars() {
                    assert!(ctx.fonts_mut(|x| x.has_glyph(&id, ch)), "{f:?} {fam:?} sem {ch}");
                }
            }
        }
    }
}
