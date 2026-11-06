// SPDX-License-Identifier: LGPL-2.1-or-later
//! Desempenho do quadro (B347): medição por etapa, limite de quadros e contador na tela.
//!
//! Três coisas moram aqui, todas sem custo quando desligadas:
//!
//! * **Medição.** Com `FENKRA_MEDIR_QUADROS=<arquivo.csv>` cada etapa do quadro (`pre`, `paineis`,
//!   `viewport`, `pos`), o intervalo entre quadros e o tempo de ponta a ponta do `ui` vão para o
//!   mesmo CSV de [`crate::medir_quadros`]. `FENKRA_MEDIR_GPU=1` espera a placa depois de desenhar a
//!   viewport e registra `gpu_vp` (muda o tempo medido, porque tira o paralelismo entre CPU e placa).
//!   `FENKRA_DEMO_GIRAR=<graus por quadro>` gira a câmera sozinha, como um arrasto contínuo, e
//!   `FENKRA_MEDIR_FIM=<n>` fecha o programa depois de n quadros.
//! * **Limite de quadros.** [`Limite`] guarda a escolha da pessoa; [`esperar_limite`] dorme o que
//!   falta do intervalo mínimo no fim do quadro. Parado não há quadro, então não há o que dormir.
//! * **Contador.** [`Contador`] mede os quadros por segundo reais (intervalo entre quadros) e diz
//!   "em repouso" quando o app deixou de redesenhar.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ medição

/// Cronômetro de etapas do quadro. Cada [`marca`] fecha a etapa aberta desde a marca anterior.
struct Relogio {
    ultima: Instant,
    inicio: Instant,
}

static RELOGIO: std::sync::Mutex<Option<Relogio>> = std::sync::Mutex::new(None);

/// A medição de etapas está ligada?
pub fn medindo() -> bool {
    crate::medir_quadros::ativo()
}

/// Começo do `ui`: abre o cronômetro e registra o intervalo desde o quadro anterior.
pub fn inicio_quadro() {
    if !medindo() {
        return;
    }
    let agora = Instant::now();
    let mut g = RELOGIO.lock().unwrap();
    if let Some(a) = g.as_ref().map(|r| r.inicio) {
        crate::medir_quadros::valor("intervalo_ms", (agora - a).as_secs_f64() * 1000.);
    }
    *g = Some(Relogio { ultima: agora, inicio: agora });
}

/// Fecha o quadro (registra o total) quando sai de escopo, mesmo nos `return` antecipados.
pub struct Guarda;

impl Drop for Guarda {
    fn drop(&mut self) {
        fim_quadro();
    }
}

/// Abre o quadro e devolve a guarda que o fecha.
pub fn quadro_guardado() -> Guarda {
    inicio_quadro();
    Guarda
}

/// Fecha a etapa que vinha correndo e dá a ela o `nome`.
pub fn marca(nome: &str) {
    if !medindo() {
        return;
    }
    let mut g = RELOGIO.lock().unwrap();
    if let Some(r) = g.as_mut() {
        let agora = Instant::now();
        crate::medir_quadros::valor(&format!("etapa_{nome}_ms"), (agora - r.ultima).as_secs_f64() * 1000.);
        r.ultima = agora;
    }
}

/// Fim do `ui`: tempo total da lógica da interface neste quadro.
pub fn fim_quadro() {
    if !medindo() {
        return;
    }
    let g = RELOGIO.lock().unwrap();
    if let Some(r) = g.as_ref() {
        crate::medir_quadros::valor("ui_total_ms", r.inicio.elapsed().as_secs_f64() * 1000.);
    }
}

/// Registra que as preferências mudaram no quadro e quais chaves de primeiro nível mudaram.
pub fn settings_mudou(antes: &crate::settings::Settings, depois: &crate::settings::Settings) {
    if !medindo() {
        return;
    }
    crate::medir_quadros::valor("settings_mudou", 1.);
    if let (Ok(a), Ok(b)) = (serde_json::to_value(antes), serde_json::to_value(depois)) {
        if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            for (k, v) in b {
                if a.get(k) != Some(v) {
                    crate::medir_quadros::valor(&format!("settings_chave_{k}"), 1.);
                }
            }
        }
    }
}

/// Espera a placa terminar o que foi enviado e registra quanto levou (`FENKRA_MEDIR_GPU=1`).
pub fn esperar_placa(device: &eframe::egui_wgpu::wgpu::Device) {
    static LIGADO: OnceLock<bool> = OnceLock::new();
    if !medindo() || !*LIGADO.get_or_init(|| std::env::var_os("FENKRA_MEDIR_GPU").is_some()) {
        return;
    }
    let t = Instant::now();
    let _ = device.poll(eframe::egui_wgpu::wgpu::PollType::wait_indefinitely());
    crate::medir_quadros::valor("gpu_vp_ms", t.elapsed().as_secs_f64() * 1000.);
}

/// `FENKRA_DEMO_GIRAR=<graus por quadro>`: quanto a câmera gira sozinha em cada quadro.
pub fn giro_automatico() -> Option<f32> {
    static G: OnceLock<Option<f32>> = OnceLock::new();
    *G.get_or_init(|| std::env::var("FENKRA_DEMO_GIRAR").ok().and_then(|v| v.trim().parse().ok()))
}

/// `FENKRA_MEDIR_FIM=<n>`: fecha o programa depois de n quadros.
pub fn fechar_se_acabou(ctx: &eframe::egui::Context) {
    static N: OnceLock<Option<u64>> = OnceLock::new();
    let Some(n) = *N.get_or_init(|| std::env::var("FENKRA_MEDIR_FIM").ok().and_then(|v| v.trim().parse().ok())) else { return };
    if crate::partida::quadros() >= n {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
    }
}

// ------------------------------------------------------------------ limite de quadros

/// Limite de quadros por segundo escolhido nas configurações gráficas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Limite {
    Q30,
    #[default]
    Q60,
    Q120,
    Q144,
    SemLimite,
}

impl Limite {
    pub const ALL: [Limite; 5] = [Limite::Q30, Limite::Q60, Limite::Q120, Limite::Q144, Limite::SemLimite];

    /// Quadros por segundo; `None` é sem limite.
    pub fn fps(self) -> Option<u32> {
        match self {
            Limite::Q30 => Some(30),
            Limite::Q60 => Some(60),
            Limite::Q120 => Some(120),
            Limite::Q144 => Some(144),
            Limite::SemLimite => None,
        }
    }

    /// Menor intervalo entre o começo de dois quadros seguidos.
    pub fn intervalo(self) -> Option<Duration> {
        self.fps().map(|f| Duration::from_secs_f64(1. / f as f64))
    }

    pub fn label(self) -> &'static str {
        use crate::i18n::tr;
        match self {
            Limite::Q30 => tr("30 quadros por segundo"),
            Limite::Q60 => tr("60 quadros por segundo"),
            Limite::Q120 => tr("120 quadros por segundo"),
            Limite::Q144 => tr("144 quadros por segundo"),
            Limite::SemLimite => tr("Sem limite"),
        }
    }
}

/// Início do último quadro que passou pelo limitador.
static ULTIMO_INICIO_US: AtomicU64 = AtomicU64::new(NUNCA);
const NUNCA: u64 = u64::MAX;
static ORIGEM: OnceLock<Instant> = OnceLock::new();

fn agora_us() -> u64 {
    ORIGEM.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// Quanto falta para o quadro que começou em `inicio_us` cumprir o intervalo mínimo.
/// Separado para o teste conferir a conta sem dormir.
pub fn falta_para_o_limite(limite: Limite, inicio_anterior_us: u64, agora_us: u64) -> Duration {
    let Some(min) = limite.intervalo() else { return Duration::ZERO };
    let min_us = min.as_micros() as u64;
    let passou = agora_us.saturating_sub(inicio_anterior_us);
    // depois de uma parada longa (app em repouso) o intervalo já passou: nada a dormir
    Duration::from_micros(min_us.saturating_sub(passou))
}

/// Chamada no começo do `ui`. Se o quadro veio cedo demais para o limite, dorme o que falta,
/// contado a partir do começo do quadro anterior. Sem limite, ou depois de repouso, não dorme.
/// Devolve quanto dormiu (para o contador e para o teste).
pub fn esperar_limite(limite: Limite) -> Duration {
    let agora = agora_us();
    let anterior = ULTIMO_INICIO_US.load(Ordering::Relaxed);
    // o primeiro quadro do programa não tem quadro anterior para esperar
    let falta = if anterior == NUNCA { Duration::ZERO } else { falta_para_o_limite(limite, anterior, agora) };
    if !falta.is_zero() {
        dormir(falta);
    }
    ULTIMO_INICIO_US.store(agora_us(), Ordering::Relaxed);
    falta
}

/// Dorme até `d`; o último milissegundo é de espera ativa, porque o sono do Windows erra por
/// frações de milissegundo e isso aparece como quadro irregular.
fn dormir(d: Duration) {
    let fim = Instant::now() + d;
    if d > Duration::from_millis(2) {
        std::thread::sleep(d - Duration::from_millis(1));
    }
    while Instant::now() < fim {
        std::hint::spin_loop();
    }
}

// ------------------------------------------------------------------ sincronismo vertical

/// Configuração da superfície para o sincronismo pedido. A latência máxima de 2 quadros é a do
/// padrão do egui para interfaces com bastante trabalho de placa, como esta viewport.
pub fn superficie(vsync: bool) -> eframe::egui_wgpu::SurfaceConfig {
    use eframe::egui_wgpu::wgpu::PresentMode;
    eframe::egui_wgpu::SurfaceConfig { present_mode: if vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync }, desired_maximum_frame_latency: Some(2) }
}

/// Liga ou desliga o sincronismo vertical na hora. Só mexe na superfície quando a escolha muda, e o
/// egui só a reconfigura nesse caso: com o app parado isso não pede quadro nenhum.
pub fn aplicar_vsync(frame: &mut eframe::Frame, vsync: bool) {
    let quer = superficie(vsync);
    if frame.wgpu_surface_config().is_some_and(|atual| atual != quer) {
        frame.set_wgpu_surface_config(quer);
    }
}

// ------------------------------------------------------------------ contador

/// Quadros por segundo reais, para mostrar na tela.
#[derive(Default)]
pub struct Contador {
    /// instantes (segundos do relógio do egui) dos quadros da rajada atual, no último segundo
    quadros: std::collections::VecDeque<f64>,
    /// último valor calculado
    pub fps: f32,
}

/// Janela do contador, em segundos.
const JANELA_S: f64 = 1.0;
/// Uma pausa deste tamanho sem quadros separa uma rajada de outra e conta como repouso.
const REPOUSO_S: f64 = 0.5;

impl Contador {
    /// Registra um quadro em `agora` (segundos) e atualiza o valor. Depois de uma pausa de repouso
    /// a medição recomeça: o primeiro quadro de uma rajada não mede nada sozinho.
    pub fn quadro(&mut self, agora: f64) {
        if self.quadros.back().is_some_and(|t| agora - *t >= REPOUSO_S) {
            self.quadros.clear();
        }
        self.quadros.push_back(agora);
        while self.quadros.front().is_some_and(|t| agora - *t > JANELA_S) {
            self.quadros.pop_front();
        }
        self.fps = match (self.quadros.front(), self.quadros.back()) {
            (Some(a), Some(b)) if self.quadros.len() > 1 && b > a => (self.quadros.len() - 1) as f32 / (b - a) as f32,
            _ => 0.,
        };
    }

    /// Sem rajada (um quadro só, ou nenhum): o app está em repouso.
    pub fn em_repouso(&self) -> bool {
        self.quadros.len() < 2
    }

    /// Daqui a quanto pedir o quadro que confere o repouso, ou `None` se o app já está em repouso.
    /// Esse é o único quadro que o contador faz o app desenhar sozinho, e só uma vez por rajada.
    pub fn proxima_conferencia(&self) -> Option<Duration> {
        (!self.em_repouso()).then(|| Duration::from_secs_f64(REPOUSO_S + 0.05))
    }

    /// O texto do contador.
    pub fn texto(&self) -> String {
        use crate::i18n::tr;
        if self.em_repouso() {
            tr("em repouso").to_owned()
        } else {
            format!("{:.0} {}", self.fps, tr("quadros/s"))
        }
    }
}

static CONTADOR: std::sync::Mutex<Option<Contador>> = std::sync::Mutex::new(None);

/// Conta o quadro que está começando. Ligado, pede o quadro de conferência de repouso; desligado,
/// não guarda nada e não pede nada.
pub fn contar_quadro(ctx: &eframe::egui::Context, ligado: bool) {
    let mut g = CONTADOR.lock().unwrap();
    if !ligado {
        *g = None;
        return;
    }
    let c = g.get_or_insert_with(Contador::default);
    c.quadro(ctx.input(|i| i.time));
    if let Some(d) = c.proxima_conferencia() {
        ctx.request_repaint_after(d);
    }
}

/// Desenha o contador no canto de baixo à direita, por cima de tudo e sem pegar o mouse.
pub fn contador_ui(ctx: &eframe::egui::Context, ligado: bool) {
    if !ligado {
        return;
    }
    let Some(texto) = CONTADOR.lock().unwrap().as_ref().map(Contador::texto) else { return };
    use eframe::egui;
    egui::Area::new(egui::Id::new("contador_de_quadros")).order(egui::Order::Foreground).anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12., -36.)).interactable(false).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new(texto).monospace());
        });
    });
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sem_limite_nao_espera() {
        assert_eq!(falta_para_o_limite(Limite::SemLimite, 0, 0), Duration::ZERO);
    }

    #[test]
    fn limite_de_60_completa_o_intervalo() {
        let f = falta_para_o_limite(Limite::Q60, 1_000_000, 1_004_000);
        let esperado = 1_000_000. / 60. - 4_000.;
        assert!((f.as_micros() as f64 - esperado).abs() < 2., "{f:?}");
    }

    #[test]
    fn quadro_lento_nao_dorme_e_repouso_nao_acumula() {
        // um quadro de 40 ms já passou do intervalo de 30 quadros por segundo
        assert_eq!(falta_para_o_limite(Limite::Q30, 0, 40_000), Duration::ZERO);
        // depois de um minuto parado o limitador não "guarda" o atraso nem dorme de novo
        assert_eq!(falta_para_o_limite(Limite::Q30, 0, 60_000_000), Duration::ZERO);
    }

    #[test]
    fn limites_e_intervalos() {
        let fps: Vec<_> = Limite::ALL.iter().map(|l| l.fps()).collect();
        assert_eq!(fps, vec![Some(30), Some(60), Some(120), Some(144), None]);
        assert!(Limite::SemLimite.intervalo().is_none());
        assert_eq!(Limite::default(), Limite::Q60);
    }

    /// O limitador de verdade dorme o que falta e deixa os quadros a, no mínimo, o intervalo pedido.
    #[test]
    fn esperar_limite_segura_os_quadros() {
        // o estado do limitador é global: este é o único teste que o usa
        let t = Instant::now();
        for _ in 0..7 {
            esperar_limite(Limite::Q30);
        }
        // 7 chamadas = 6 intervalos de 1/30 s depois da primeira
        let minimo = Duration::from_secs_f64(6. / 30.) - Duration::from_millis(2);
        assert!(t.elapsed() >= minimo, "{:?}", t.elapsed());
        // sem limite não dorme
        let t = Instant::now();
        for _ in 0..50 {
            esperar_limite(Limite::SemLimite);
        }
        assert!(t.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn contador_mede_a_rajada_e_volta_ao_repouso() {
        let mut c = Contador::default();
        assert!(c.em_repouso() && c.proxima_conferencia().is_none());
        // 60 quadros por segundo, durante meio segundo
        for i in 0..30 {
            c.quadro(i as f64 / 60.);
        }
        assert!((c.fps - 60.).abs() < 1., "{}", c.fps);
        assert!(!c.em_repouso());
        assert!(c.proxima_conferencia().is_some(), "durante a rajada pede a conferência");
        // um quadro só, bem depois: é a conferência, a rajada acabou
        c.quadro(1.6);
        assert!(c.em_repouso(), "o quadro de conferência não vira medição");
        assert_eq!(c.fps, 0.);
        assert!(c.proxima_conferencia().is_none(), "em repouso não pede mais quadro: o app volta a parar");
        assert_eq!(c.texto(), "em repouso");
    }

    /// Roda `n` quadros vazios do egui a partir de `t0` (60 por segundo) e devolve, de cada um, o
    /// menor atraso de repintura pedido.
    fn atrasos(ctx: &eframe::egui::Context, t0: f64, n: usize, ligado: bool) -> Vec<Duration> {
        (0..n)
            .map(|i| {
                let entrada = eframe::egui::RawInput { time: Some(t0 + i as f64 / 60.), ..Default::default() };
                let mut saida = ctx.run_ui(entrada, |ui| contar_quadro(ui.ctx(), ligado));
                saida.textures_delta.clear();
                saida.viewport_output.values().map(|v| v.repaint_delay).min().unwrap_or(Duration::MAX)
            })
            .collect()
    }

    #[test]
    fn contador_desligado_nao_pede_quadro() {
        let ctx = eframe::egui::Context::default();
        // os primeiros quadros do egui se pedem sozinhos (fontes, layout); depois ele fica quieto
        let a = atrasos(&ctx, 0., 12, false);
        assert!(a[8..].iter().all(|d| *d == Duration::MAX), "o egui já devia estar quieto: {a:?}");
    }

    #[test]
    fn contador_ligado_pede_so_a_conferencia() {
        let ctx = eframe::egui::Context::default();
        let _ = atrasos(&ctx, 0., 12, false);
        // ligado, em movimento: cada quadro pede a conferência de repouso para daqui a ~0,55 s
        let a = atrasos(&ctx, 1., 6, true);
        // o primeiro quadro de uma rajada ainda não mede nada, então não pede conferência
        assert_eq!(a[0], Duration::MAX, "{a:?}");
        assert!(a[1..].iter().all(|d| *d >= Duration::from_millis(400) && *d < Duration::from_secs(2)), "{a:?}");
        // desligar devolve o app ao repouso total, sem nenhum quadro pedido
        let _ = atrasos(&ctx, 2., 6, false);
        let a = atrasos(&ctx, 3., 6, false);
        assert!(a.iter().all(|d| *d == Duration::MAX), "{a:?}");
    }

    #[test]
    fn superficie_acompanha_o_vsync() {
        use eframe::egui_wgpu::wgpu::PresentMode;
        assert_eq!(superficie(true).present_mode, PresentMode::AutoVsync);
        assert_eq!(superficie(false).present_mode, PresentMode::AutoNoVsync);
    }
}
