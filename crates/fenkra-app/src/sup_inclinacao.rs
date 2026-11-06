// SPDX-License-Identifier: LGPL-2.1-or-later
//! B180: mapa de inclinação (ângulo de saída) e o tipo de cor por vértice da cena.
//!
//! A cor de cada vértice vem do ângulo entre a face e a direção de abertura: `asin(|n·d|)`, 0 numa
//! parede paralela à abertura e 90 numa tampa perpendicular a ela. Quatro faixas, com as cores de
//! estado do tema (a mesma leitura de "bom" e "ruim" do resto do programa):
//! abaixo do mínimo (erro), até o dobro do mínimo (aviso), saída suficiente (ok) e face quase
//! perpendicular à abertura (destaque). O mínimo é o limite que o perfil de processo usa (B197).

use crate::app::App;
use crate::i18n::tr;
use crate::sup_analise::{paradas, rampa, EscalaCurvatura};
use eframe::egui::{self, Rect, RichText};

/// A partir daqui a face conta como "tampa": perpendicular o bastante à abertura para não pedir saída.
pub const TAMPA_GRAUS: f32 = 60.;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EscalaInclinacao {
    /// direção de abertura, unitária
    pub dir: [f32; 3],
    /// ângulo de saída mínimo, em graus
    pub minimo: f32,
    pub paradas: [[f32; 3]; 4],
}

impl EscalaInclinacao {
    /// Ângulo de saída de uma normal, em graus (0 a 90).
    pub fn angulo(&self, n: [f32; 3]) -> f32 {
        let c = n[0] * self.dir[0] + n[1] * self.dir[1] + n[2] * self.dir[2];
        let m = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
        (c.abs() / m).clamp(0., 1.).asin().to_degrees()
    }

    /// Posição da faixa na escala do tema (0 destaque, 1/3 ok, 2/3 aviso, 1 erro).
    pub fn faixa(&self, ang: f32) -> f32 {
        if ang < self.minimo {
            1.
        } else if ang < 2. * self.minimo {
            2. / 3.
        } else if ang < TAMPA_GRAUS {
            1. / 3.
        } else {
            0.
        }
    }

    pub fn cor(&self, n: [f32; 3]) -> [f32; 3] {
        rampa(&self.paradas, self.faixa(self.angulo(n)))
    }

    pub fn cor_da_malha(&self, m: &fenkra_kernel::Mesh, i: usize) -> Option<[f32; 3]> {
        let k = i * 3;
        Some(self.cor([*m.normals.get(k)?, *m.normals.get(k + 1)?, *m.normals.get(k + 2)?]))
    }
}

/// O que a cena pinta por vértice: curvatura ou inclinação.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EscalaMapa {
    Curvatura(EscalaCurvatura),
    Inclinacao(EscalaInclinacao),
}

impl From<EscalaCurvatura> for EscalaMapa {
    fn from(e: EscalaCurvatura) -> Self {
        EscalaMapa::Curvatura(e)
    }
}

impl EscalaMapa {
    pub fn cor_da_malha(&self, m: &fenkra_kernel::Mesh, i: usize) -> Option<[f32; 3]> {
        match self {
            EscalaMapa::Curvatura(e) => e.cor_da_malha(m, i),
            EscalaMapa::Inclinacao(e) => e.cor_da_malha(m, i),
        }
    }
}

/// Direção de abertura escolhida nas preferências.
pub fn direcao(eixo: u8, inverte: bool) -> [f32; 3] {
    let mut d = [0.; 3];
    d[(eixo as usize).min(2)] = if inverte { -1. } else { 1. };
    d
}

/// Escala em uso: curvatura, inclinação ou nenhuma.
pub fn escala_do_mapa(s: &crate::settings::Settings, v: &egui::Visuals) -> Option<EscalaMapa> {
    match s.display {
        crate::render::DisplayMode::Curvatura => crate::sup_analise::escala_da_tela(s, v).map(Into::into),
        crate::render::DisplayMode::Inclinacao => Some(EscalaMapa::Inclinacao(EscalaInclinacao { dir: direcao(s.incl_eixo, s.incl_inverte), minimo: s.incl_min.max(0.1), paradas: paradas(v) })),
        _ => None,
    }
}

/// Painel livre da escala, aberto só enquanto a tela está no mapa de inclinação.
pub fn painel_inclinacao(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Encaixe, Painel, Posicao};
    if app.settings.display != crate::render::DisplayMode::Inclinacao {
        return;
    }
    let cfg = Painel { nome: "escala_inclinacao".into(), titulo: tr("Mapa de inclinação").to_string(), padrao: paineis::abaixo_do_cubo(area, 240.), largura: 240., redimensiona: false };
    let posicao = app.settings.painel(&cfg.nome, Encaixe::Livre);
    let p = paradas(&ctx.global_style().visuals);
    let s = &mut app.settings;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.label(RichText::new(tr("Direção de abertura")).small());
        ui.horizontal(|ui| {
            for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
                ui.selectable_value(&mut s.incl_eixo, i as u8, *n);
            }
            ui.checkbox(&mut s.incl_inverte, tr("Inverter")).on_hover_text(tr("Troca o sentido da abertura: cavidade e núcleo trocam de lado."));
        });
        ui.add(crate::ui_ext::slider(&mut s.incl_min, 0.5..=10.).suffix("°").text(tr("Saída mínima"))).on_hover_text(tr("Ângulo de saída abaixo do qual a face fica em erro. O molde costuma pedir de 1° a 3°."));
        let m = s.incl_min;
        let faixas = [(p[3], format!("< {}°", crate::app::fmt(m as f64)), "Sem saída suficiente"), (p[2], format!("{}° a {}°", crate::app::fmt(m as f64), crate::app::fmt(2. * m as f64)), "Saída justa"), (p[1], format!("{}° a {}°", crate::app::fmt(2. * m as f64), TAMPA_GRAUS as i32), "Saída suficiente"), (p[0], format!("> {}°", TAMPA_GRAUS as i32), "Perpendicular à abertura")];
        for (cor, faixa, nome) in faixas {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(14., 10.), egui::Sense::hover());
                ui.painter().rect_filled(r, 2., crate::theme::cor_da_escala(cor));
                ui.label(RichText::new(faixa).small().strong());
                ui.label(RichText::new(tr(nome)).small().weak());
            });
        }
    });
    if let Some(m) = saida.movido {
        app.settings.guardar_painel(&cfg.nome, Posicao { ..m });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [[f32; 3]; 4] = [[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [1., 0., 0.]];

    fn esc(min: f32) -> EscalaInclinacao {
        EscalaInclinacao { dir: [0., 0., 1.], minimo: min, paradas: P }
    }

    #[test]
    fn angulo_e_zero_na_parede_e_noventa_na_tampa() {
        let e = esc(1.);
        assert!(e.angulo([1., 0., 0.]) < 1e-4);
        assert!((e.angulo([0., 0., 1.]) - 90.).abs() < 1e-3);
        assert!((e.angulo([0., 0., -1.]) - 90.).abs() < 1e-3, "o sinal não importa: o outro lado do molde");
        let t = 3f32.to_radians();
        assert!((e.angulo([t.cos(), 0., t.sin()]) - 3.).abs() < 1e-3);
    }

    #[test]
    fn quatro_faixas_pelas_cores_de_estado() {
        let e = esc(2.);
        let t = |g: f32| [g.to_radians().cos(), 0., g.to_radians().sin()];
        assert_eq!(e.cor(t(0.)), P[3], "parede reta: erro");
        assert_eq!(e.cor(t(1.5)), P[3]);
        assert_eq!(e.cor(t(3.)), rampa(&P, 2. / 3.), "justa");
        assert_eq!(e.cor(t(10.)), rampa(&P, 1. / 3.), "suficiente");
        assert_eq!(e.cor(t(80.)), P[0], "tampa");
    }

    #[test]
    fn inverter_nao_muda_a_cor_so_o_lado_do_molde() {
        let a = EscalaInclinacao { dir: direcao(2, false), ..esc(1.) };
        let b = EscalaInclinacao { dir: direcao(2, true), ..esc(1.) };
        let n = [0.3, 0.2, 0.93];
        assert_eq!(a.cor(n), b.cor(n));
        assert_eq!(direcao(0, true), [-1., 0., 0.]);
    }
}
