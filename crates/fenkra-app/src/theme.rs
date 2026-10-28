//! Temas da interface: cores dos painéis, janelas, texto, destaque, faixas da barra e fundo da
//! viewport. Sete temas prontos e os do usuário (arquivos .json em
//! %APPDATA%\Fenkra\temas — uma extensão é só um arquivo desses).

use eframe::egui::{self, Color32, Stroke};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

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
}

fn t(name: &str, dark: bool, panel: [u8; 3], window: [u8; 3], field: [u8; 3], text: [u8; 3], accent: [u8; 3], band_a: [u8; 3], band_b: [u8; 3], bg_top: [u8; 3], bg_bottom: [u8; 3]) -> ThemeDef {
    ThemeDef { name: name.into(), dark, panel, window, field, text, accent, band_a, band_b, bg_top, bg_bottom }
}

/// Temas prontos.
pub fn builtin() -> Vec<ThemeDef> {
    vec![
        t("Escuro", true, [27, 29, 33], [32, 34, 39], [14, 15, 18], [225, 228, 234], [70, 140, 230], [30, 32, 37], [40, 43, 50], [74, 84, 100], [26, 29, 34]),
        t("Claro", false, [238, 240, 243], [248, 249, 251], [255, 255, 255], [24, 28, 34], [30, 110, 220], [228, 231, 236], [240, 242, 246], [214, 222, 232], [158, 170, 186]),
        t("NeoTech Minimalista", false, [244, 246, 249], [255, 255, 255], [255, 255, 255], [32, 38, 48], [59, 130, 246], [236, 240, 246], [247, 249, 252], [247, 249, 252], [210, 218, 230]),
        t("Dark Fusion", true, [12, 15, 29], [18, 23, 42], [7, 9, 18], [222, 226, 245], [124, 92, 255], [16, 20, 38], [26, 22, 52], [36, 40, 82], [6, 7, 15]),
        t("Industrial Titanium", true, [43, 45, 49], [54, 57, 63], [28, 29, 32], [226, 228, 232], [232, 163, 58], [50, 52, 57], [62, 65, 71], [104, 108, 116], [30, 31, 34]),
        t("AeroVision", false, [226, 238, 251], [242, 248, 255], [255, 255, 255], [13, 34, 56], [30, 136, 229], [214, 230, 248], [232, 242, 253], [207, 227, 247], [111, 163, 214]),
        t("CyberCAD", true, [7, 20, 38], [11, 29, 54], [3, 10, 20], [210, 236, 255], [0, 180, 255], [9, 26, 48], [12, 38, 70], [15, 43, 82], [2, 8, 18]),
    ]
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

fn c(x: [u8; 3]) -> Color32 {
    Color32::from_rgb(x[0], x[1], x[2])
}

fn mix(a: [u8; 3], b: [u8; 3], k: f32) -> Color32 {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
    Color32::from_rgb(m(0), m(1), m(2))
}

/// Aplica o tema na interface.
pub fn apply(ctx: &egui::Context, def: &ThemeDef) {
    let mut v = if def.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = c(def.panel);
    v.window_fill = c(def.window);
    v.extreme_bg_color = c(def.field);
    v.faint_bg_color = mix(def.panel, def.text, 0.05);
    v.code_bg_color = mix(def.panel, def.text, 0.08);
    v.override_text_color = Some(c(def.text));
    v.hyperlink_color = c(def.accent);
    v.selection.bg_fill = mix(def.panel, def.accent, if def.dark { 0.55 } else { 0.35 });
    v.selection.stroke = Stroke::new(1., c(def.accent));
    v.window_stroke = Stroke::new(1., mix(def.panel, def.text, 0.16));
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.menu_corner_radius = egui::CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c(def.panel);
    w.noninteractive.weak_bg_fill = c(def.panel);
    w.noninteractive.bg_stroke = Stroke::new(1., mix(def.panel, def.text, 0.12));
    w.noninteractive.fg_stroke = Stroke::new(1., mix(def.panel, def.text, 0.85));
    // campos e caixas de marcar sempre visíveis: preenchimento e contorno com contraste
    w.inactive.bg_fill = mix(def.panel, def.text, if def.dark { 0.16 } else { 0.10 });
    w.inactive.weak_bg_fill = mix(def.panel, def.text, 0.07);
    w.inactive.bg_stroke = Stroke::new(1., mix(def.panel, def.text, if def.dark { 0.30 } else { 0.28 }));
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
        x.corner_radius = egui::CornerRadius::same(6);
    }
    ctx.set_visuals(v);
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(def.clone());
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
    p.circle_filled(cc, 9., Color32::from_rgb(178, 180, 226));
    p.circle_stroke(cc, 9., Stroke::new(1., Color32::from_black_alpha(160)));
    p.text(egui::pos2(r.center().x, r.bottom() - 9.), egui::Align2::CENTER_CENTER, &def.name, egui::FontId::proportional(10.5), c(def.text));
    let stroke = if selected { Stroke::new(2.5, c(def.accent)) } else if hover { Stroke::new(1.5, mix(def.panel, def.accent, 0.6)) } else { Stroke::new(1., mix(def.panel, def.text, 0.2)) };
    p.rect_stroke(r, 8., stroke, egui::StrokeKind::Inside);
    resp
}

fn dark_now() -> bool {
    current().is_none_or(|d| d.dark)
}

/// Cores de estado legíveis no tema em uso (ok, aviso, erro).
pub fn st_ok() -> Color32 {
    if dark_now() { Color32::from_rgb(110, 210, 120) } else { Color32::from_rgb(30, 130, 50) }
}
pub fn st_warn() -> Color32 {
    if dark_now() { Color32::from_rgb(240, 200, 90) } else { Color32::from_rgb(170, 110, 0) }
}
pub fn st_err() -> Color32 {
    if dark_now() { Color32::from_rgb(255, 120, 100) } else { Color32::from_rgb(200, 40, 30) }
}
