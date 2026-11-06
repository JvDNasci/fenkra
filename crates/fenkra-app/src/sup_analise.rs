// SPDX-License-Identifier: LGPL-2.1-or-later
//! B34: o que as análises de superfície mostram. Os números vêm prontos do cálculo; aqui eles viram cor
//! e etiqueta. A escala usa as cores de estado do tema: a mesma leitura de "bom" e "ruim" do resto do
//! programa, em qualquer tema.

use crate::app::App;
use crate::i18n::tr;
use crate::render::{Camera, MeshVertex, Scene};
use eframe::egui::{self, Color32, Rect, RichText};
use fenkra_core::*;
use fenkra_solidos::analise::{Analise, Conexao, Dente, Desvio, Nivel, TOL_ANGULO, TOL_FOLGA};
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

fn rgb(c: Color32) -> [f32; 3] {
    [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255.]
}

/// Cor do tema na posição `t` (0 a 1) da escala de quatro paradas.
pub fn rampa(paradas: &[[f32; 3]; 4], t: f32) -> [f32; 3] {
    let x = t.clamp(0., 1.) * 3.;
    let i = (x.floor() as usize).min(2);
    let f = x - i as f32;
    [0, 1, 2].map(|k| paradas[i][k] * (1. - f) + paradas[i + 1][k] * f)
}

/// Escala do tema, da superfície plana (destaque) ao problema (erro).
pub fn paradas(v: &egui::Visuals) -> [[f32; 3]; 4] {
    [rgb(crate::theme::accent(v)), rgb(crate::theme::st_ok()), rgb(crate::theme::st_warn()), rgb(crate::theme::st_err())]
}

/// O que o mapa de curvatura pinta.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TipoCurvatura {
    /// a maior curvatura em valor absoluto: acha o menor raio
    #[default]
    Maxima,
    /// média das duas principais
    Media,
    /// produto das duas: zero em superfície desenvolvível
    Gaussiana,
}

impl TipoCurvatura {
    pub const TODOS: [TipoCurvatura; 3] = [TipoCurvatura::Maxima, TipoCurvatura::Media, TipoCurvatura::Gaussiana];

    pub fn rotulo(self) -> &'static str {
        match self {
            TipoCurvatura::Maxima => "Máxima",
            TipoCurvatura::Media => "Média",
            TipoCurvatura::Gaussiana => "Gaussiana",
        }
    }

    /// Valor em 1/mm. A gaussiana sai como raiz, para ter a mesma unidade e a mesma escala das outras.
    pub fn valor(self, k1: f32, k2: f32) -> f32 {
        match self {
            TipoCurvatura::Maxima => k1.abs().max(k2.abs()),
            TipoCurvatura::Media => ((k1 + k2) / 2.).abs(),
            TipoCurvatura::Gaussiana => (k1 * k2).abs().sqrt(),
        }
    }
}

/// Escala do mapa de curvatura para a cena.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EscalaCurvatura {
    pub tipo: TipoCurvatura,
    /// raio que recebe a última cor da escala, em mm
    pub raio: f32,
    pub paradas: [[f32; 3]; 4],
}

impl EscalaCurvatura {
    /// Cor do vértice. A escala é logarítmica: um raio dez vezes maior anda a mesma distância em
    /// qualquer ponto dela, e o raio grande não some perto do plano.
    pub fn cor(&self, k1: f32, k2: f32) -> [f32; 3] {
        let k = self.tipo.valor(k1, k2) * self.raio;
        rampa(&self.paradas, (1. + k * 15.).log2() / 4.)
    }

    /// Cor do vértice `i` da malha.
    pub fn cor_da_malha(&self, m: &fenkra_kernel::Mesh, i: usize) -> Option<[f32; 3]> {
        Some(self.cor(*m.curvature.get(i * 2)?, *m.curvature.get(i * 2 + 1)?))
    }
}

/// Escala em uso, se a tela está no mapa de curvatura.
pub fn escala_da_tela(s: &crate::settings::Settings, v: &egui::Visuals) -> Option<EscalaCurvatura> {
    (s.display == crate::render::DisplayMode::Curvatura).then(|| EscalaCurvatura { tipo: s.curv_tipo, raio: s.curv_raio, paradas: paradas(v) })
}

/// Barra de cores com os rótulos das pontas.
fn barra(ui: &mut egui::Ui, p: &[[f32; 3]; 4], ini: &str, fim: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.), egui::Sense::hover());
    let n = 48;
    for i in 0..n {
        let c = crate::theme::cor_da_escala(rampa(p, i as f32 / (n - 1) as f32));
        let (x0, x1) = (r.left() + r.width() * i as f32 / n as f32, r.left() + r.width() * (i + 1) as f32 / n as f32);
        ui.painter().rect_filled(Rect::from_min_max(egui::pos2(x0, r.top()), egui::pos2(x1 + 0.5, r.bottom())), 0., c);
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new(ini).small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(fim).small()));
    });
}

/// Painel livre da escala, aberto só enquanto a tela está no mapa de curvatura.
pub fn painel_curvatura(app: &mut App, ctx: &egui::Context, area: Rect) {
    use crate::paineis::{self, Encaixe, Painel, Posicao};
    if app.settings.display != crate::render::DisplayMode::Curvatura {
        return;
    }
    let cfg = Painel { nome: "escala_curvatura".into(), titulo: tr("Escala de curvatura").to_string(), padrao: paineis::abaixo_do_cubo(area, 220.), largura: 220., redimensiona: false };
    let posicao = app.settings.painel(&cfg.nome, Encaixe::Livre);
    let p = paradas(&ctx.global_style().visuals);
    let s = &mut app.settings;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.horizontal(|ui| {
            for t in TipoCurvatura::TODOS {
                ui.selectable_value(&mut s.curv_tipo, t, tr(t.rotulo()));
            }
        });
        ui.add(crate::ui_ext::slider(&mut s.curv_raio, 0.5..=2000.).logarithmic(true).suffix(" mm").text(tr("Raio"))).on_hover_text(tr("Raio que recebe a última cor. A medida vem da superfície: não muda com o zoom nem com a malha."));
        barra(ui, &p, tr("plano"), &format!("R {} mm", crate::app::fmt(s.curv_raio as f64)));
    });
    if let Some(m) = saida.movido {
        app.settings.guardar_painel(&cfg.nome, Posicao { ..m });
    }
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Cor do desvio: em cima de zero, a primeira cor; no maior desvio, a última.
fn pintar_desvio(d: &Desvio, p: &[[f32; 3]; 4], s: &mut Scene) {
    let m = &*d.malha;
    let maior = d.min.abs().max(d.max.abs()).max(1e-9) as f32;
    for &vi in &m.indices {
        let i = vi as usize;
        let k = i * 3;
        let c = rampa(p, d.valores.get(i).map_or(0., |v| v.abs() / maior));
        s.ghost.push(MeshVertex { pos: [m.positions[k], m.positions[k + 1], m.positions[k + 2]], nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [c[0], c[1], c[2], 0.95], mat: [1., 0., 1., 0.] });
    }
}

/// Marca de uma amostra: quanto pior, mais perto da última cor.
fn nota_da_amostra(a: &fenkra_kernel::AmostraConexao) -> f32 {
    ((a.folga / TOL_FOLGA).max(a.angulo / TOL_ANGULO) as f32 / 4.).clamp(0., 1.)
}

fn pintar_conexao(c: &Conexao, p: &[[f32; 3]; 4], s: &mut Scene) {
    let w: Vec<Vec3> = c.amostras.iter().map(|a| v3(a.ponto)).collect();
    for (i, a) in c.amostras.iter().enumerate() {
        let k = rampa(p, nota_da_amostra(a));
        let col = [k[0], k[1], k[2], 1.];
        if let Some(b) = w.get(i + 1).filter(|b| (**b - w[i]).length() < 1e3) {
            crate::render::push_wide(&mut s.wide, &[w[i].to_array(), b.to_array()], col, 5., 0);
        }
    }
}

fn pintar_pente(d: &[Dente], p: &[[f32; 3]; 4], s: &mut Scene) {
    let topo = d.iter().fold(1e-12f64, |m, x| m.max(x.k));
    let col = |k: f64| {
        let c = rampa(p, (k / topo) as f32);
        [c[0], c[1], c[2], 1.]
    };
    for x in d {
        crate::render::push_wide(&mut s.wide, &[v3(x.base).to_array(), v3(x.ponta).to_array()], col(x.k), 1.5, 0);
    }
    for w in d.windows(2) {
        crate::render::push_wide(&mut s.wide, &[v3(w[0].ponta).to_array(), v3(w[1].ponta).to_array()], col(w[0].k), 2., 0);
    }
}

/// Dentes do pente na cena: serve à análise e às curvas editadas.
pub fn pente_na_cena(d: &[Dente], v: &egui::Visuals, s: &mut Scene) {
    pintar_pente(d, &paradas(v), s);
}

/// Mapa e marcas da análise aberta.
pub fn decorar(r: &RecomputeResult, id: Id, p: &[[f32; 3]; 4], s: &mut Scene) {
    match r.notes.get(&id).and_then(|n| n.analise.as_ref()) {
        Some(Analise::Conexao(c)) => pintar_conexao(c, p, s),
        Some(Analise::Desvio(d)) => pintar_desvio(d, p, s),
        Some(Analise::Pente(d)) => pintar_pente(d, p, s),
        None => {}
    }
}

fn nome_do_nivel(n: Nivel) -> &'static str {
    match n {
        Nivel::Aberta => "Aberta",
        Nivel::G0 => "G0: encosta",
        Nivel::G1 => "G1: tangente",
        Nivel::G2 => "G2: curvatura",
    }
}

/// Linhas de número do painel do comando.
pub fn resumo(ui: &mut egui::Ui, a: Option<&Analise>) {
    let Some(a) = a else { return };
    let v = ui.visuals().clone();
    let linha = |ui: &mut egui::Ui, n: &str, x: String| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(n)).small());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(x).strong()));
        });
    };
    match a {
        Analise::Conexao(c) => {
            linha(ui, "Junção", tr(nome_do_nivel(c.nivel())).to_string());
            linha(ui, "Folga máxima", format!("{:.4} mm", c.folga));
            linha(ui, "Ângulo máximo", format!("{:.3}°", c.angulo));
            linha(ui, "Δ curvatura", format!("{:.5} 1/mm", c.dif_curvatura));
            linha(ui, "Amostras", c.amostras.len().to_string());
            barra(ui, &paradas(&v), tr("dentro"), tr("fora"));
        }
        Analise::Desvio(d) => {
            linha(ui, "Mínimo", format!("{:.4} mm", d.min));
            linha(ui, "Máximo", format!("{:.4} mm", d.max));
            linha(ui, "RMS", format!("{:.4} mm", d.rms));
            barra(ui, &paradas(&v), "0", &format!("{:.3} mm", d.min.abs().max(d.max.abs())));
        }
        Analise::Pente(d) => {
            if let Some(t) = fenkra_solidos::analise::pico(d) {
                linha(ui, "Maior curvatura", format!("{:.5} 1/mm", t.k));
                if t.k > 1e-9 {
                    linha(ui, "Menor raio", format!("{:.3} mm", 1. / t.k));
                }
            }
        }
    }
}

/// Etiqueta no ponto do pior caso.
pub fn overlay(app: &App, ui: &egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    let Some(r) = app.result.as_ref() else { return };
    let Some(n) = r.notes.get(&id).filter(|n| n.analise.is_some()) else { return };
    let Some(p) = r.points.iter().find(|p| p.id == id) else { return };
    let Some(q) = cam.project(rect, v3(p.p)) else { return };
    let pintor = ui.painter_at(rect);
    let cor = crate::theme::st_err();
    pintor.circle_stroke(q, 7., egui::Stroke::new(2., cor));
    let t = pintor.layout_no_wrap(n.text.clone(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), cor);
    // a etiqueta fica sempre dentro da vista: o pior caso perto da borda a deixava cortada
    let tam = t.size() + egui::vec2(8., 4.);
    let canto = q + egui::vec2(12., -8.);
    let min = egui::pos2(canto.x.min(rect.right() - tam.x - 4.).max(rect.left() + 4.), canto.y.min(rect.bottom() - tam.y - 26.).max(rect.top() + 4.));
    let caixa = Rect::from_min_size(min, tam);
    pintor.rect_filled(caixa, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.9));
    pintor.galley(caixa.min + egui::vec2(4., 2.), t, cor);
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [[f32; 3]; 4] = [[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [1., 0., 0.]];

    #[test]
    fn rampa_cobre_as_paradas() {
        assert_eq!(rampa(&P, 0.), P[0]);
        assert_eq!(rampa(&P, 1.), P[3]);
        assert_eq!(rampa(&P, 2.), P[3]);
    }

    #[test]
    fn cor_da_curvatura_depende_so_da_superficie_e_da_escala() {
        let e = EscalaCurvatura { tipo: TipoCurvatura::Maxima, raio: 10., paradas: P };
        assert_eq!(e.cor(0., 0.), P[0], "plano");
        assert_eq!(e.cor(0.1, 0.), P[3], "raio igual ao da escala fica na última cor");
        assert_eq!(e.cor(0.2, 0.), P[3]);
        assert!(e.cor(0.01, 0.)[0] < e.cor(0.05, 0.)[0], "raio menor anda na escala");
    }

    #[test]
    fn tipos_de_curvatura() {
        assert_eq!(TipoCurvatura::Maxima.valor(0.1, -0.3), 0.3);
        assert_eq!(TipoCurvatura::Media.valor(0.1, -0.1), 0.);
        assert_eq!(TipoCurvatura::Gaussiana.valor(0.5, 0.), 0., "cilindro é desenvolvível");
    }
}
