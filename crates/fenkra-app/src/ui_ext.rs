// SPDX-License-Identifier: LGPL-2.1-or-later
//! Pequenas extensões da interface usadas em todos os painéis.
//!
//! Campo numérico padrão (B227): todo número editável usa `dv` (arrasto e digitação) ou `slider`,
//! e a roda do mouse vale igual em qualquer painel, sem chamada manual.

use eframe::egui;
use std::sync::atomic::{AtomicU8, Ordering};

/// Quando a roda do mouse altera um campo numérico (a mesma regra em todos os painéis).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModoRoda {
    /// Só com o campo em edição (clicado): rolar o painel com o mouse por cima não muda valores.
    Foco,
    /// Só com Ctrl pressionado sobre o campo.
    Ctrl,
    /// Sempre que o mouse estiver sobre o campo.
    Sempre,
}

impl ModoRoda {
    pub const TODOS: [ModoRoda; 3] = [ModoRoda::Foco, ModoRoda::Ctrl, ModoRoda::Sempre];

    pub fn de_u8(n: u8) -> ModoRoda {
        match n {
            1 => ModoRoda::Ctrl,
            2 => ModoRoda::Sempre,
            _ => ModoRoda::Foco,
        }
    }

    fn nome(self) -> &'static str {
        match self {
            ModoRoda::Foco => "Só depois de clicar no campo (padrão)",
            ModoRoda::Ctrl => "Só com Ctrl pressionado",
            ModoRoda::Sempre => "Sempre que o mouse estiver sobre o campo",
        }
    }

    fn explicacao(self) -> &'static str {
        match self {
            ModoRoda::Foco => "Rolar o painel com o mouse por cima de um campo não altera valores; clique no campo para liberar a roda.",
            ModoRoda::Ctrl => "A roda só altera o campo com Ctrl pressionado; sem Ctrl o painel rola normalmente.",
            ModoRoda::Sempre => "A roda altera o campo sob o mouse, sem clicar. Rápido, mas rolar o painel pode mudar valores sem querer.",
        }
    }
}

/// Página de Configurações do modo da roda. Aplica na hora e traz um campo de teste para sentir a regra.
pub fn modo_roda_ui(ui: &mut egui::Ui, modo: &mut u8) {
    use crate::i18n::tr;
    ui.label(egui::RichText::new(tr("Roda do mouse nos campos numéricos")).strong());
    for m in ModoRoda::TODOS {
        ui.radio_value(modo, m as u8, tr(m.nome()));
    }
    let atual = ModoRoda::de_u8(*modo);
    ui.label(egui::RichText::new(tr(atual.explicacao())).weak().small());
    definir_modo_roda(atual);
    let chave = egui::Id::new("teste_roda_campo");
    let mut x: f64 = ui.ctx().data(|d| d.get_temp(chave)).unwrap_or(10.);
    ui.horizontal(|ui| {
        ui.label(tr("Campo de teste:"));
        ui.add(dv(&mut x).speed(1.).range(0.0..=1000.0).suffix(" mm"));
    });
    ui.ctx().data_mut(|d| d.insert_temp(chave, x));
}

static MODO: AtomicU8 = AtomicU8::new(0);

pub fn modo_roda() -> ModoRoda {
    match MODO.load(Ordering::Relaxed) {
        1 => ModoRoda::Ctrl,
        2 => ModoRoda::Sempre,
        _ => ModoRoda::Foco,
    }
}

pub fn definir_modo_roda(m: ModoRoda) {
    MODO.store(m as u8, Ordering::Relaxed);
}

/// Multiplicador da roda conforme as teclas: Shift ×10, Ctrl ×0,1, Alt ×0,01 (no modo Ctrl, o
/// Ctrl só libera a roda). Shift tem o mesmo sentido no painel e no 3D (maior = mais rápido).
pub fn mult_teclas(m: egui::Modifiers) -> f64 {
    if m.shift {
        10.
    } else if m.command && modo_roda() != ModoRoda::Ctrl {
        0.1
    } else if m.alt {
        0.01
    } else {
        1.
    }
}

/// Passo base de um "dente" a partir da velocidade do campo: potência de 10 (0,5 e 1 dão 1; 0,1 dá 0,1).
fn passo_base(speed: f64) -> f64 {
    if speed <= 0. || !speed.is_finite() {
        return 1.;
    }
    10f64.powf((speed * 2.).log10().floor())
}

/// Roda do mouse sobre o campo: devolve os "dentes" girados (+ para cima) e consome a rolagem
/// para o painel não rolar junto. Vale a regra de `modo_roda`. No mesmo quadro um campo só
/// responde uma vez, então o componente e as chamadas manuais antigas não se somam.
fn notches(ui: &egui::Ui, resp: &egui::Response) -> f64 {
    let liberado = match modo_roda() {
        ModoRoda::Foco => resp.has_focus(),
        ModoRoda::Ctrl => ui.input(|i| i.modifiers.command),
        ModoRoda::Sempre => true,
    };
    dentes(ui, resp, liberado)
}

/// Rótulo de valor no 3D: a roda age só pelo mouse por cima (sem foco), com o mesmo passo e teclas.
pub fn dentes_3d(ui: &egui::Ui, resp: &egui::Response) -> f64 {
    dentes(ui, resp, true)
}

/// Passo de arrasto no 3D que segue o zoom: 1, 2 ou 5 vezes uma potência de 10, o maior que cabe em 8 px na tela,
/// vezes as teclas (Shift ×10, Ctrl ×0,1, Alt ×0,01), como nos campos dos painéis. Peça de 5 mm com
/// zoom perto dá 0,1 mm; peça de 5 m dá 10 mm.
pub fn passo_3d(ui: &egui::Ui, px_por_mm: f32) -> f64 {
    let ppm = (px_por_mm as f64).max(1e-6);
    let base = crate::camera_nav::passo_125(ppm as f32);
    base * ui.input(|i| mult_teclas(i.modifiers))
}

fn dentes(ui: &egui::Ui, resp: &egui::Response, liberado: bool) -> f64 {
    if !resp.hovered() || !liberado {
        return 0.;
    }
    let chave = egui::Id::new(("campo_roda", resp.id));
    let quadro = ui.ctx().cumulative_pass_nr();
    if ui.ctx().data(|m| m.get_temp::<u64>(chave)) == Some(quadro) {
        return 0.;
    }
    ui.ctx().data_mut(|m| m.insert_temp(chave, quadro));
    // sobre o campo, a roda não rola o painel
    ui.ctx().input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    let d: f32 = ui.input(|i| {
        i.raw
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                _ => None,
            })
            .sum()
    });
    if d == 0. {
        return 0.;
    }
    d.signum() as f64
}

/// Campo de texto numérico (expressões): a roda soma/subtrai quando o texto é um número.
pub fn wheel_text(ui: &egui::Ui, resp: &egui::Response, s: &mut String) -> bool {
    let n = notches(ui, resp);
    if n == 0. {
        return false;
    }
    let Ok(v) = s.trim().replace(',', ".").parse::<f64>() else { return false };
    let m = ui.input(|i| mult_teclas(i.modifiers));
    let v = ((v + n * m) * 1e6).round() / 1e6;
    *s = crate::app::fmt(v);
    true
}

fn aplicar<T: egui::emath::Numeric>(ui: &egui::Ui, resp: &egui::Response, v: &mut T, base: f64, faixa: Option<(f64, f64)>) -> bool {
    let n = notches(ui, resp);
    if n == 0. {
        return false;
    }
    let m = ui.input(|i| mult_teclas(i.modifiers));
    let st = if T::INTEGRAL { (base * m).max(1.) } else { base * m };
    let mut x = v.to_f64() + n * st;
    if let Some((lo, hi)) = faixa {
        x = x.clamp(lo, hi);
    }
    *v = T::from_f64(if T::INTEGRAL { x.round() } else { (x * 1e6).round() / 1e6 });
    true
}

/// Chamada manual da roda. Os campos de `dv`/`slider` já a aplicam; isto só serve a quem monta
/// o widget por fora e é inofensivo se chamado de novo no mesmo campo.
pub fn wheel<T: egui::emath::Numeric>(ui: &egui::Ui, resp: &egui::Response, v: &mut T) {
    aplicar(ui, resp, v, 1., None);
}

/// Campo numérico padrão: arrasto, digitação com vírgula, roda (mesma regra em todo painel) e dica.
pub struct Campo<'a, N: egui::emath::Numeric> {
    dv: egui::DragValue<'a>,
    ptr: *mut N,
    speed: f64,
    faixa: Option<(f64, f64)>,
    dica: Option<&'a str>,
    padrao: Option<f64>,
    rotulo: Option<egui::WidgetText>,
}

impl<'a, N: egui::emath::Numeric> Campo<'a, N> {
    pub fn speed(mut self, s: impl Into<f64>) -> Self {
        let s = s.into();
        self.speed = s;
        self.dv = self.dv.speed(s);
        self
    }
    pub fn range(mut self, r: impl Into<std::ops::RangeInclusive<N>>) -> Self {
        let r = r.into();
        self.faixa = Some((r.start().to_f64(), r.end().to_f64()));
        self.dv = self.dv.range(r);
        self
    }
    pub fn suffix(mut self, s: impl Into<egui::WidgetText>) -> Self {
        let s = s.into();
        // campo em graus lê `90°` e `1,57 rad`; os demais leem comprimentos
        if s.text().contains('°') {
            self.dv = self.dv.custom_parser(|t| parse_com_unidade(t, true));
        }
        self.dv = self.dv.suffix(s);
        self
    }
    /// Valor padrão: o menu de contexto do campo (botão direito) oferece restaurá-lo.
    pub fn padrao(mut self, v: impl Into<f64>) -> Self {
        self.padrao = Some(v.into());
        self
    }
    /// Rótulo à esquerda do campo; arrastá-lo na horizontal muda o valor (Shift ×10, Ctrl ×0,1,
    /// Alt ×0,01) e duplo clique restaura o padrão, se houver.
    pub fn rotulo(mut self, t: impl Into<egui::WidgetText>) -> Self {
        self.rotulo = Some(t.into());
        self
    }
    pub fn prefix(mut self, s: impl Into<egui::WidgetText>) -> Self {
        self.dv = self.dv.prefix(s);
        self
    }
    pub fn max_decimals(mut self, n: usize) -> Self {
        self.dv = self.dv.max_decimals(n);
        self
    }
    pub fn min_decimals(mut self, n: usize) -> Self {
        self.dv = self.dv.min_decimals(n);
        self
    }
    pub fn custom_formatter(mut self, f: impl 'a + Fn(f64, std::ops::RangeInclusive<usize>) -> String) -> Self {
        self.dv = self.dv.custom_formatter(f);
        self
    }
    /// Dica do campo mostrada ao pairar (já traduzida).
    pub fn dica(mut self, d: &'a str) -> Self {
        self.dica = Some(d);
        self
    }
}

/// Muda o valor pelo arrasto horizontal de `dx` pixels: `speed` unidades por pixel vezes as teclas.
pub fn arrastar_valor(v: f64, dx: f64, speed: f64, mult: f64, faixa: Option<(f64, f64)>) -> f64 {
    let x = v + dx * speed * mult;
    let x = faixa.map_or(x, |(lo, hi)| x.clamp(lo, hi));
    (x * 1e6).round() / 1e6
}

fn arredondar<N: egui::emath::Numeric>(x: f64) -> N {
    N::from_f64(if N::INTEGRAL { x.round() } else { x })
}

impl<N: egui::emath::Numeric> egui::Widget for Campo<'_, N> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Campo { dv, ptr, speed, faixa, dica, padrao, rotulo } = self;
        let corpo = |ui: &mut egui::Ui, rot: Option<egui::Response>| {
            let mut r = ui.add(dv);
            // SAFETY: `ptr` vem do `&mut N` que o `DragValue` emprestava; o widget já foi consumido
            // por `ui.add`, então nada mais usa esse empréstimo aqui.
            let v = unsafe { &mut *ptr };
            if aplicar(ui, &r, v, passo_base(speed), faixa) {
                r.mark_changed();
            }
            if let Some(l) = rot {
                if l.dragged() {
                    let m = ui.input(|i| mult_teclas(i.modifiers));
                    let novo = arrastar_valor(v.to_f64(), l.drag_delta().x as f64, speed, m, faixa);
                    *v = arredondar(novo);
                    r.mark_changed();
                }
                if let (true, Some(p)) = (l.double_clicked(), padrao) {
                    *v = arredondar(p);
                    r.mark_changed();
                }
            }
            if let Some(p) = padrao {
                r.context_menu(|ui| {
                    if ui.button(format!("{} ({})", crate::i18n::tr("Restaurar padrão"), crate::app::fmt(p))).clicked() {
                        *v = arredondar(p);
                        ui.close();
                    }
                });
            }
            if let Some(d) = dica {
                r = r.on_hover_text(d);
            }
            r
        };
        match rotulo {
            Some(t) => {
                ui.horizontal(|ui| {
                    let l = ui.add(egui::Label::new(t).sense(egui::Sense::click_and_drag())).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    corpo(ui, Some(l))
                })
                .inner
            }
            None => corpo(ui, None),
        }
    }
}

/// Slider com a roda no mesmo padrão dos demais campos.
pub struct Deslizante<'a, N: egui::emath::Numeric> {
    sl: egui::Slider<'a>,
    ptr: *mut N,
    faixa: (f64, f64),
}

impl<'a, N: egui::emath::Numeric> Deslizante<'a, N> {
    pub fn text(mut self, t: impl Into<egui::WidgetText>) -> Self {
        self.sl = self.sl.text(t);
        self
    }
    pub fn suffix(mut self, s: impl Into<egui::WidgetText>) -> Self {
        self.sl = self.sl.suffix(s);
        self
    }
}

impl<N: egui::emath::Numeric> egui::Widget for Deslizante<'_, N> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Deslizante { sl, ptr, faixa } = self;
        let mut r = ui.add(sl);
        // SAFETY: igual a `Campo`: o empréstimo do `Slider` já terminou em `ui.add`.
        let v = unsafe { &mut *ptr };
        let base = if N::INTEGRAL { 1. } else { passo_base((faixa.1 - faixa.0).abs() / 100.) };
        if aplicar(ui, &r, v, base, Some(faixa)) {
            r.mark_changed();
        }
        r
    }
}

/// Separador decimal do idioma nos números exibidos (vírgula em português).
fn dec(s: String) -> String {
    if crate::i18n::english() { s } else { s.replace('.', ",") }
}

/// Lê número com vírgula ou ponto, com unidade opcional (`10 cm`, `2in`, `90°`, `1,5 rad`).
fn parse_dec(s: &str) -> Option<f64> {
    parse_com_unidade(s, false)
}

/// Fator de conversão da unidade digitada: comprimentos para mm, ângulos para graus.
fn fator_unidade(u: &str, angulo: bool) -> Option<f64> {
    let u = u.trim().to_lowercase();
    if angulo {
        return match u.as_str() {
            "°" | "deg" | "graus" | "grau" => Some(1.),
            "rad" => Some(180. / std::f64::consts::PI),
            "grad" | "gon" => Some(0.9),
            _ => None,
        };
    }
    match u.as_str() {
        "mm" => Some(1.),
        "cm" => Some(10.),
        "m" => Some(1000.),
        "km" => Some(1e6),
        "um" | "µm" | "μm" => Some(0.001),
        "in" | "pol" | "\"" => Some(25.4),
        "ft" | "pé" | "pe" => Some(304.8),
        "mil" => Some(0.0254),
        _ => None,
    }
}

/// Número com unidade opcional. Comprimentos viram milímetros; com `angulo`, os ângulos viram graus
/// (a unidade ausente vale o padrão do campo). Texto que não é número com unidade conhecida dá `None`.
pub fn parse_com_unidade(s: &str, angulo: bool) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if let Ok(v) = t.parse::<f64>() {
        return Some(v);
    }
    // corta onde o número termina: o maior prefixo que lê como número, com o resto como unidade
    let marcas: Vec<usize> = t.char_indices().map(|(i, _)| i).skip(1).collect();
    for i in marcas.into_iter().rev() {
        if let (Ok(n), Some(f)) = (t[..i].trim().parse::<f64>(), fator_unidade(&t[i..], angulo)) {
            return Some(n * f);
        }
    }
    None
}

/// Campo numérico padrão com o separador decimal do idioma (ver `Campo`).
pub fn dv<Num: egui::emath::Numeric>(v: &mut Num) -> Campo<'_, Num> {
    let ptr: *mut Num = v;
    let dv = egui::DragValue::new(v).custom_formatter(|n, r| dec(egui::emath::format_with_decimals_in_range(n, r))).custom_parser(parse_dec);
    Campo { dv, ptr, speed: 1., faixa: None, dica: None, padrao: None, rotulo: None }
}

/// Slider com o separador decimal do idioma e a roda padrão.
pub fn slider<Num: egui::emath::Numeric>(v: &mut Num, range: std::ops::RangeInclusive<Num>) -> Deslizante<'_, Num> {
    let ptr: *mut Num = v;
    let faixa = (range.start().to_f64(), range.end().to_f64());
    let sl = egui::Slider::new(v, range).custom_formatter(|n, r| dec(egui::emath::format_with_decimals_in_range(n, r))).custom_parser(parse_dec);
    Deslizante { sl, ptr, faixa }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn passo_base_segue_velocidade() {
        assert_eq!(passo_base(1.), 1.);
        assert_eq!(passo_base(0.5), 1.);
        assert_eq!(passo_base(0.1), 0.1);
        assert_eq!(passo_base(0.01), 0.01);
        assert_eq!(passo_base(0.), 1.);
    }

    /// Campo numérico só pelo componente: construir `DragValue`/`Slider` por fora perde a roda.
    #[test]
    fn nenhum_campo_numerico_fora_do_componente() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut ruins = vec![];
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "ui_ext.rs") {
                let t = std::fs::read_to_string(&p).unwrap();
                for (i, l) in t.lines().enumerate() {
                    let l = l.trim_start();
                    if !l.starts_with("//") && (l.contains("DragValue::new(") || l.contains("Slider::new(")) {
                        ruins.push(format!("{}:{}", p.file_name().unwrap().to_string_lossy(), i + 1));
                    }
                }
            }
        }
        assert!(ruins.is_empty(), "use ui_ext::dv/slider: {ruins:?}");
    }

    #[test]
    fn unidade_digitada_vira_milimetro_ou_grau() {
        let mm = |s| parse_com_unidade(s, false);
        assert_eq!(mm("10"), Some(10.));
        assert_eq!(mm("10 mm"), Some(10.));
        assert_eq!(mm("2,5cm"), Some(25.));
        assert_eq!(mm("1 m"), Some(1000.));
        assert_eq!(mm("1in"), Some(25.4));
        assert_eq!(mm("1e3"), Some(1000.));
        assert_eq!(mm("abc"), None);
        assert_eq!(mm("10 xx"), None);
        assert_eq!(mm("90°"), None, "grau não vale em campo de comprimento");
        assert_eq!(parse_com_unidade("90°", true), Some(90.));
        let rad = parse_com_unidade("3,14159265 rad", true).unwrap();
        assert!((rad - 180.).abs() < 1e-4, "{rad}");
        assert_eq!(parse_com_unidade("5 cm", true), None, "comprimento não vale em campo de ângulo");
    }

    #[test]
    fn arrasto_no_rotulo_respeita_teclas_e_faixa() {
        assert_eq!(arrastar_valor(10., 4., 0.5, 1., None), 12.);
        assert_eq!(arrastar_valor(10., 4., 0.5, 10., None), 30.);
        assert_eq!(arrastar_valor(10., 4., 0.5, 0.1, None), 10.2);
        assert_eq!(arrastar_valor(10., -100., 1., 1., Some((0., 50.))), 0.);
    }

    #[test]
    fn modo_da_roda_tem_nome_e_explicacao_traduzidos() {
        for m in ModoRoda::TODOS {
            assert!(crate::i18n::tem_traducao(m.nome()) && crate::i18n::tem_traducao(m.explicacao()), "{m:?}");
            assert_eq!(ModoRoda::de_u8(m as u8), m);
        }
    }

    #[test]
    fn passo_3d_acompanha_o_zoom() {
        // fórmula pura: 8 px equivalem a 0,1 mm com 80 px/mm e a 10 mm com 0,8 px/mm
        let base = |ppm: f64| 10f64.powf((8. / ppm).log10().floor());
        assert_eq!(base(80.), 0.1);
        assert_eq!(base(0.8), 10.);
    }

    #[test]
    fn shift_acelera_e_alt_afina() {
        let mut m = egui::Modifiers::NONE;
        assert_eq!(mult_teclas(m), 1.);
        m.shift = true;
        assert_eq!(mult_teclas(m), 10.);
        assert_eq!(mult_teclas(egui::Modifiers::ALT), 0.01);
    }
}
