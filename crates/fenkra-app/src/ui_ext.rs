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

/// Passo de arrasto no 3D que segue o zoom: potência de 10 equivalente a cerca de 8 px na tela,
/// vezes as teclas (Shift ×10, Ctrl ×0,1, Alt ×0,01), como nos campos dos painéis. Peça de 5 mm com
/// zoom perto dá 0,1 mm; peça de 5 m dá 10 mm.
pub fn passo_3d(ui: &egui::Ui, px_por_mm: f32) -> f64 {
    let ppm = (px_por_mm as f64).max(1e-6);
    let base = 10f64.powf((8. / ppm).log10().floor()).clamp(0.001, 1000.);
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
        self.dv = self.dv.suffix(s);
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

impl<N: egui::emath::Numeric> egui::Widget for Campo<'_, N> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Campo { dv, ptr, speed, faixa, dica } = self;
        let mut r = ui.add(dv);
        // SAFETY: `ptr` vem do `&mut N` que o `DragValue` emprestava; o widget já foi consumido por
        // `ui.add`, então nada mais usa esse empréstimo aqui.
        let v = unsafe { &mut *ptr };
        if aplicar(ui, &r, v, passo_base(speed), faixa) {
            r.mark_changed();
        }
        if let Some(d) = dica {
            r = r.on_hover_text(d);
        }
        r
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

/// Lê número com vírgula ou ponto.
fn parse_dec(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

/// Campo numérico padrão com o separador decimal do idioma (ver `Campo`).
pub fn dv<Num: egui::emath::Numeric>(v: &mut Num) -> Campo<'_, Num> {
    let ptr: *mut Num = v;
    let dv = egui::DragValue::new(v).custom_formatter(|n, r| dec(egui::emath::format_with_decimals_in_range(n, r))).custom_parser(parse_dec);
    Campo { dv, ptr, speed: 1., faixa: None, dica: None }
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
