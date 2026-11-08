// SPDX-License-Identifier: LGPL-2.1-or-later
//! A vista pede a malha de tela (B346, ADR 0069): quando a câmera para e o cálculo termina, compara o
//! que a tela precisa com o que já pediu e, se mudou, manda o pedido para a thread de cálculo. Órbita não
//! muda nada; zoom só conta ao cruzar um degrau de `Polimento::para_vista`.

use super::App;
use crate::render::DisplayMode;
use eframe::egui;
use fenkra_core::Polimento;
use std::time::{Duration, Instant};

/// Espera depois do último resultado: arrastar uma alça manda um cálculo atrás do outro, e nesse
/// intervalo a vista não deve disputar a thread com a edição.
const ESPERA: Duration = Duration::from_millis(250);

/// O que vai junto de cada pedido de cálculo.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pedido {
    pub alvo: Option<Polimento>,
    /// tessela agora; sem isso o cálculo só reaproveita o que tem
    pub agora: bool,
    /// comando aberto para edição: a rosca modelada dele vira só linhas até concluir (B351)
    pub leve: Option<fenkra_core::Id>,
}

#[derive(Default)]
pub struct Polidor {
    /// último alvo pedido e a geração em que foi pedido
    feito: Option<(Option<Polimento>, u64)>,
    alvo: Option<Polimento>,
    ultimo_resultado: Option<Instant>,
}

impl Polidor {
    /// Pedido de quem edita: não tessela, e o corpo que não mudou segue com a malha que tem.
    pub fn pedido_de_edicao(&self) -> Pedido {
        Pedido { alvo: self.alvo, agora: false, leve: None }
    }

    pub fn resultado_chegou(&mut self, r: &fenkra_solidos::RecomputeResult) {
        self.ultimo_resultado = Some(Instant::now());
        if crate::medir_quadros::ativo() {
            crate::medir_quadros::valor("triangulos_tela", r.exibicao.values().map(|m| m.indices.len() / 3).sum::<usize>() as f64);
            crate::medir_quadros::valor("triangulos_trabalho", r.bodies.iter().map(|b| b.mesh.indices.len() / 3).sum::<usize>() as f64);
            crate::medir_quadros::valor("calculo_ms", r.elapsed_ms);
        }
    }
}

/// Modos que comparam a superfície com ela mesma: precisam do ângulo fino.
fn e_analise(d: DisplayMode) -> bool {
    matches!(d, DisplayMode::Zebra | DisplayMode::Curvatura | DisplayMode::Inclinacao | DisplayMode::Isofotas)
}

impl App {
    /// O que a tela precisa agora. Nada na montagem, que escolhe o nível de cada cópia na placa (ADR
    /// 0053), e nada quando há pintura feita sobre a malha de trabalho: ela ficaria por baixo da de tela.
    fn alvo_da_vista(&self, altura_px: u32) -> Option<Polimento> {
        let d = self.settings.display;
        let pinta_sobre_a_malha = d == DisplayMode::Dfm || self.result.as_ref().is_some_and(|r| r.notes.values().any(|n| matches!(n.analise, Some(fenkra_solidos::analise::Analise::Desvio(_)))));
        // `FENKRA_DEMO_SEM_POLIMENTO`: fica na malha de trabalho, para comparar antes e depois
        if pinta_sobre_a_malha || matches!(d, DisplayMode::LinhasOcultas | DisplayMode::Arame) || std::env::var_os("FENKRA_DEMO_SEM_POLIMENTO").is_some() {
            return None;
        }
        let c = self.viewport.as_ref()?.camera;
        Polimento::para_vista(c.px_por_mm(c.dist, altura_px as f32) as f64, e_analise(d), d == DisplayMode::Curvatura, self.settings.modo_leve)
    }

    /// Chamado a cada quadro.
    pub(super) fn polir_vista(&mut self, ctx: &egui::Context) {
        if self.in_asm() || self.calculando() || self.sent_doc.is_none() {
            return;
        }
        let Some(vp) = self.viewport.as_ref() else { return };
        let Some(altura) = vp.altura_px() else { return };
        let parada = !vp.navegando() && !vp.animando();
        let alvo = self.alvo_da_vista(altura);
        if self.polir.feito == Some((alvo, self.gen_done)) {
            return;
        }
        let espera = self.polir.ultimo_resultado.map_or(Duration::ZERO, |t| ESPERA.saturating_sub(t.elapsed()));
        if !parada || !espera.is_zero() {
            // a interface só redesenha quando algo muda: sem isto o pedido ficaria para o próximo evento
            ctx.request_repaint_after(espera.max(crate::render::PAUSA_NAVEGACAO));
            return;
        }
        self.polir.feito = Some((alvo, self.gen_done));
        self.polir.alvo = alvo;
        // B304: na animação de dobrar, a malha fina é a da peça parcial que está na tela
        let Some(doc) = self.doc_enviado() else { return };
        self.worker.enviar((self.gen_sent, doc, false, self.qualidade_malha, Pedido { alvo, agora: true, leve: self.session.as_ref().map(|s| s.id) }));
    }
}
