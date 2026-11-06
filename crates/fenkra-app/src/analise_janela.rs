// SPDX-License-Identifier: LGPL-2.1-or-later
//! B357: o painel de cada análise (curvatura, inclinação, fabricação, zebra, isofotas) fecha.
//!
//! Causa: o painel existe enquanto a tela está no modo de análise (`Settings::display`), mas só o
//! seletor de exibição saía desse modo. O painel não tinha botão de fechar, e como o modo vai para as
//! preferências ele voltava a cada abertura do programa. Agora todo painel de análise ganha um `✕`
//! na barra de título: ele devolve a tela ao sombreado com arestas, o que fecha o painel e tira o
//! mapa de cores da peça. O `✕` fica numa camada própria sobre o painel, então não mexe no encaixe.

use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::DisplayMode;
use crate::settings::Settings;
use eframe::egui::{self, Rect};

/// Modo para onde a tela volta ao fechar a análise.
pub const MODO_NORMAL: DisplayMode = DisplayMode::SombreadoArestas;

/// Fecha a análise: sai do modo, e o painel deixa de ser desenhado no próximo quadro.
pub fn fechar(s: &mut Settings) {
    if s.display.e_analise() {
        s.display = MODO_NORMAL;
    }
}

/// `✕` sobre o canto direito da barra de título do painel. Devolve se foi clicado.
fn botao_fechar(ctx: &egui::Context, rect: Rect, nome: &str) -> bool {
    let lado = 18.;
    // painel recolhido (só a barra de título): o título ocupa a largura toda, então o ✕ fica ao lado
    let recolhido = rect.height() < 48.;
    let x = if recolhido { rect.right() + 4. } else { rect.right() - lado - 6. };
    let pos = egui::pos2(x, rect.top() + 4.);
    egui::Area::new(egui::Id::unique(("fechar_analise", nome)))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| ui.add_sized([lado, lado], egui::Button::new(egui::RichText::new("✕").small()).frame(false)).on_hover_text(tr("Fechar a análise")).clicked())
        .inner
}

/// Painel de análise com `✕`. O conteúdo recebe as preferências; o resto (posição guardada, fechar)
/// é daqui. Devolve o retângulo do painel neste quadro.
pub fn janela(ctx: &egui::Context, area: Rect, cfg: &Painel, s: &mut Settings, conteudo: impl FnOnce(&mut egui::Ui, &mut Settings)) -> Option<Rect> {
    let posicao = s.painel(&cfg.nome, Encaixe::Livre);
    let saida = paineis::mostrar(ctx, area, cfg, posicao, |ui| conteudo(ui, &mut *s));
    if let Some(m) = saida.movido {
        s.guardar_painel(&cfg.nome, m);
    }
    if saida.rect.is_some_and(|r| botao_fechar(ctx, r, &cfg.nome)) {
        fechar(s);
    }
    saida.rect
}

/// Zebra e isofotas não têm número para ajustar: o painel explica o que se vê e deixa fechar.
pub fn painel_listras(app: &mut crate::app::App, ctx: &egui::Context, area: Rect) {
    let (nome, titulo, texto) = match app.settings.display {
        DisplayMode::Zebra => ("analise_zebra", "Análise: zebra", "Listras de reflexo: se elas se quebram numa junção, as superfícies só se encostam; se seguem sem torcer, a transição é suave."),
        DisplayMode::Isofotas => ("analise_isofotas", "Análise: isofotas", "Linhas de mesmo ângulo com a luz: curvas que se emendam sem dobra indicam continuidade de tangente."),
        _ => return,
    };
    let cfg = Painel { nome: nome.into(), titulo: tr(titulo).to_string(), padrao: paineis::abaixo_do_cubo(area, 240.), largura: 240., redimensiona: false };
    janela(ctx, area, &cfg, &mut app.settings, |ui, _| {
        ui.label(egui::RichText::new(tr(texto)).small());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    fn tela() -> Rect {
        Rect::from_min_size(pos2(0., 0.), vec2(1200., 800.))
    }

    fn cfg(nome: &str) -> Painel {
        Painel { nome: nome.into(), titulo: "Análise".into(), padrao: pos2(800., 100.), largura: 240., redimensiona: false }
    }

    fn quadro(ctx: &egui::Context, c: &Painel, s: &mut Settings, evs: Vec<egui::Event>) -> Option<Rect> {
        let raw = egui::RawInput { screen_rect: Some(tela()), events: evs, ..Default::default() };
        let mut r = None;
        let mut saida = ctx.run_pass(raw, |ctx| {
            // como no app: só existe com a tela num modo de análise
            if s.display.e_analise() {
                r = janela(ctx, tela(), c, s, |ui, _| {
                    ui.label("escala");
                });
            }
        });
        saida.textures_delta.clear();
        r
    }

    fn clique(p: egui::Pos2, apertado: bool) -> Vec<egui::Event> {
        vec![egui::Event::PointerMoved(p), egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: apertado, modifiers: Default::default() }]
    }

    /// Abre cada modo de análise e fecha pelo `✕` com o mouse simulado: a tela volta ao sombreado e
    /// o painel some no quadro seguinte.
    #[test]
    fn todo_modo_de_analise_abre_e_fecha_pelo_x() {
        let modos: Vec<DisplayMode> = DisplayMode::ALL.iter().copied().filter(|m| m.e_analise()).collect();
        assert_eq!(modos.len(), 5, "zebra, curvatura, inclinação, isofotas e fabricação");
        for m in modos {
            let ctx = egui::Context::default();
            let c = cfg("teste");
            let mut s = Settings::default();
            s.display = m;
            let mut r = None;
            for _ in 0..3 {
                r = quadro(&ctx, &c, &mut s, vec![]);
            }
            let r = r.unwrap_or_else(|| panic!("{m:?}: o painel abre"));
            let x = pos2(r.right() - 6. - 9., r.top() + 4. + 9.);
            quadro(&ctx, &c, &mut s, clique(x, false)[..1].to_vec());
            quadro(&ctx, &c, &mut s, clique(x, true)[1..].to_vec());
            quadro(&ctx, &c, &mut s, clique(x, false)[1..].to_vec());
            assert_eq!(s.display, MODO_NORMAL, "{m:?}: o clique no ✕ fecha a análise");
            assert!(quadro(&ctx, &c, &mut s, vec![]).is_none(), "{m:?}: o painel some");
        }
    }

    #[test]
    fn fechar_nao_mexe_em_modo_que_nao_e_de_analise() {
        let mut s = Settings::default();
        s.display = DisplayMode::Arame;
        fechar(&mut s);
        assert_eq!(s.display, DisplayMode::Arame);
    }
}
