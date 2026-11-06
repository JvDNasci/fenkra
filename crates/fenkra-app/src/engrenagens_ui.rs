// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gerador de engrenagens: prévia 3D do par, esquema do dente e modo Dimensionar (B280).
//!
//! - **Prévia**: as duas peças aparecem translúcidas na posição em que `inserir_par_engrenagens` as
//!   põe (a conta é a mesma: `fenkra_montagem::engrenagens::previa_par`). Nada é gravado em disco; as
//!   malhas saem do recálculo em memória dos documentos das peças. Os círculos primitivos são
//!   tracejados, com d e a anotados, e uma alça na face do pinhão muda a largura.
//! - **Esquema do dente**: perfil de alguns dentes com as circunferências da, d, db e df; passar o
//!   mouse num rótulo do cálculo realça a grandeza no desenho.
//! - **Dimensionar**: distância entre centros e razão dão módulo e dentes sugeridos.

use crate::app::{fmt, App};
use crate::asm_ui::{AsmState, GearsUi};
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use fenkra_kernel::Mesh;
use fenkra_montagem::engrenagens::{previa_par, PreviaPar};
use fenkra_solidos::engrenagens_dim::sugerir_par;
use fenkra_solidos::{engrenagem_documento, recompute, CalculoEngrenagem, ParamEngrenagem, TipoEngrenagem};
use glam::Vec3;
use std::sync::Arc;

/// Opacidade das duas engrenagens fantasma.
const ALFA: f32 = 0.42;

/// Malhas do par, calculadas em memória para a chave dos parâmetros que as geram.
pub struct MalhasPar {
    pub chave: String,
    pub pinhao: Mesh,
    pub coroa: Mesh,
}

/// Grandezas do cálculo que o esquema do dente sabe realçar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grandeza {
    Primitivo,
    Base,
    Topo,
    Raiz,
    Passo,
    Espessura,
    Altura,
}

impl Grandeza {
    /// A grandeza de um rótulo da grade do cálculo (texto em português, como na janela).
    pub fn do_rotulo(r: &str) -> Option<Grandeza> {
        Some(match r {
            "Diâmetro primitivo d" => Grandeza::Primitivo,
            "Diâmetro base db" => Grandeza::Base,
            "Diâmetro de topo da" => Grandeza::Topo,
            "Diâmetro de raiz df" => Grandeza::Raiz,
            "Passo circular" => Grandeza::Passo,
            "Espessura do dente" => Grandeza::Espessura,
            "Altura do dente" => Grandeza::Altura,
            _ => return None,
        })
    }
}

fn id_hover() -> egui::Id {
    egui::Id::new("engrenagem_esquema_hover")
}

/// Guarda a grandeza sob o mouse na grade do cálculo; o esquema a realça no quadro seguinte.
pub fn marcar(ctx: &egui::Context, rotulo: &str) {
    if let Some(g) = Grandeza::do_rotulo(rotulo) {
        ctx.data_mut(|d| d.insert_temp(id_hover(), g));
        ctx.request_repaint();
    }
}

/// Chave do que a malha depende: tudo que muda a forma das duas peças.
fn chave_do_par(g: &GearsUi) -> String {
    format!("{:?}|{}|{}", g.p.tipo, g.p.chave(), g.dentes_coroa)
}

fn malha_da_peca(p: &ParamEngrenagem) -> Result<Mesh, String> {
    let doc = engrenagem_documento(p)?;
    let r = recompute(&doc);
    r.bodies.into_iter().next().map(|b| b.mesh).ok_or_else(|| "sem sólido".to_string())
}

/// Refaz as malhas da prévia quando os parâmetros mudam. Durante um arrasto de campo (botão do mouse
/// pressionado) mantém as antigas: o recálculo das duas peças espera o mouse soltar. Devolve se a
/// cena precisa ser refeita.
pub fn atualizar_malhas(g: &mut GearsUi, ctx: &egui::Context) -> bool {
    let chave = chave_do_par(g);
    if g.malhas.as_ref().is_some_and(|m| m.chave == chave) {
        return false;
    }
    if ctx.input(|i| i.pointer.any_down()) && g.malhas.is_some() {
        return false;
    }
    g.malhas = previa_par(&g.p, g.dentes_coroa)
        .and_then(|pv| Ok(MalhasPar { chave, pinhao: malha_da_peca(&pv.par.pinhao)?, coroa: malha_da_peca(&pv.par.coroa)? }))
        .ok()
        .map(Arc::new);
    true
}

/// Peças da prévia como fantasma na cena, nas posições finais.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, cena: &mut crate::render::Scene) {
    let Some(g) = st.and_then(|s| s.gears_open.as_ref()).filter(|g| !g.eixo_grafico) else { return };
    let (Some(m), Ok(pv)) = (g.malhas.as_ref(), previa_par(&g.p, g.dentes_coroa)) else { return };
    let rgb = |c: Color32| [c.r(), c.g(), c.b()];
    for (malha, lugar, cor) in [(&m.pinhao, &pv.lugar_pinhao, crate::theme::accent(v)), (&m.coroa, &pv.lugar_coroa, crate::theme::st_ok())] {
        let t = fenkra_montagem::view::transformed(malha, lugar);
        crate::previa::malha_fantasma(&mut cena.ghost, &t, rgb(cor), ALFA);
    }
}

/// Polilinha tracejada de um círculo no plano z = `z` do pinhão (ou da coroa), projetada na tela.
fn circulo_tracejado(p: &egui::Painter, rect: egui::Rect, cam: &Camera, lugar: &fenkra_montagem::Placement, r: f64, z: f64, stroke: Stroke) -> Option<Pos2> {
    const N: usize = 96;
    let pts: Vec<Option<Pos2>> = (0..=N)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / N as f64;
            let q = lugar.point([r * a.cos(), r * a.sin(), z]);
            cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32))
        })
        .collect();
    // 2 de cada 4 segmentos: traço e vazio
    for i in 0..N {
        if i % 4 < 2 {
            if let (Some(a), Some(b)) = (pts[i], pts[i + 1]) {
                p.line_segment([a, b], stroke);
            }
        }
    }
    pts[N / 8].or(pts[0])
}

fn texto_com_fundo(p: &egui::Painter, ancora: Pos2, texto: String, cor: Color32, fundo: Color32) {
    let g = p.layout_no_wrap(texto, egui::FontId::proportional(12.), cor);
    p.rect_filled(egui::Rect::from_min_size(ancora, g.size()).expand(2.), 3., fundo);
    p.galley(ancora, g, cor);
}

/// Anotações da prévia: círculos primitivos tracejados com d, distância entre centros a e a alça
/// de largura na face do pinhão.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(g) = app.asm.as_mut().and_then(|s| s.gears_open.as_mut()).filter(|g| !g.eixo_grafico) else { return };
    let Ok(pv) = previa_par(&g.p, g.dentes_coroa) else { return };
    let v = ui.visuals().clone();
    let (cor, fundo) = (crate::theme::accent(&v), v.window_fill().gamma_multiply(0.85));
    let p = ui.painter_at(rect);
    let meio = g.p.largura / 2.;
    let proj = |q: [f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
    let traco = Stroke::new(1.2, cor);
    let rack = pv.par.coroa.tipo == TipoEngrenagem::Cremalheira;
    let mut centros = vec![];
    for (lugar, calc, nome) in [(&pv.lugar_pinhao, &pv.par.calc_pinhao, "d1"), (&pv.lugar_coroa, &pv.par.calc_coroa, "d2")] {
        if calc.diametro_primitivo <= 0. {
            continue;
        }
        let r = calc.diametro_primitivo / 2.;
        if let Some(ponto) = circulo_tracejado(&p, rect, cam, lugar, r, meio, traco) {
            texto_com_fundo(&p, ponto + Vec2::new(6., -6.), format!("{nome} = {}", fmt(calc.diametro_primitivo)), cor, fundo);
        }
        if let Some(c) = proj(lugar.point([0., 0., meio])) {
            p.circle_filled(c, 2.5, cor);
            centros.push(c);
        }
    }
    // distância entre centros (par externo e interno)
    if let (false, [c1, c2]) = (rack, centros.as_slice()) {
        p.line_segment([*c1, *c2], Stroke::new(1., cor.gamma_multiply(0.8)));
        texto_com_fundo(&p, *c1 + (*c2 - *c1) * 0.5 + Vec2::new(6., 6.), format!("a = {}", fmt(pv.par.distancia_centros)), cor, fundo);
    }
    // alça na face do pinhão: arrastar ao longo do eixo muda a largura
    let (Some(base), Some(topo)) = (proj(pv.lugar_pinhao.point([0., 0., 0.])), proj(pv.lugar_pinhao.point([0., 0., g.p.largura]))) else { return };
    let na_tela = topo - base;
    let px_por_mm = na_tela.length() / g.p.largura.max(1e-6) as f32;
    let id = egui::Id::new("alca_largura_engrenagem");
    let resp = ui.interact(egui::Rect::from_center_size(topo, egui::vec2(18., 18.)), id, egui::Sense::click_and_drag());
    let cor_alca = if resp.hovered() || resp.dragged() { crate::theme::st_warn() } else { crate::theme::st_ok() };
    p.circle_filled(topo, 6.5, cor_alca);
    p.circle_stroke(topo, 6.5, Stroke::new(1.5, v.window_stroke().color));
    texto_com_fundo(&p, topo + Vec2::new(12., -22.), format!("b = {}", fmt(g.p.largura)), cor_alca, fundo);
    let ctx = ui.ctx().clone();
    if resp.drag_started() {
        ctx.data_mut(|d| d.insert_temp(id, g.p.largura));
    }
    if resp.dragged() {
        let alvo = crate::alca_cota::alvo_do_arrasto(ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(g.p.largura), resp.drag_delta(), na_tela, px_por_mm);
        ctx.data_mut(|d| d.insert_temp(id, alvo));
        g.p.largura = largura_arredondada(alvo);
    }
}

/// Largura que a alça aceita: múltiplos de 0,5 mm, nunca abaixo de 0,5.
pub fn largura_arredondada(alvo: f64) -> f64 {
    ((alvo * 2.).round() / 2.).clamp(0.5, 1000.)
}

// ---------------------------------------------------------------- esquema do dente

/// Ponto do perfil (x radial, y tangencial, em mm) na tela do esquema: o raio cresce para cima, o
/// centro do dente 0 fica no meio. `r_ref` é o raio que vai ao centro vertical.
pub fn ponto_do_esquema(pt: [f64; 2], r_ref: f64, escala: f64, centro: Pos2) -> Pos2 {
    Pos2::new(centro.x + (pt[1] * escala) as f32, centro.y - ((pt[0] - r_ref) * escala) as f32)
}

fn arco_no_esquema(p: &egui::Painter, r: f64, meio_angulo: f64, r_ref: f64, escala: f64, centro: Pos2, stroke: Stroke, tracejado: bool) {
    const N: usize = 40;
    let pts: Vec<Pos2> = (0..=N)
        .map(|i| {
            let a = -meio_angulo + 2. * meio_angulo * i as f64 / N as f64;
            ponto_do_esquema([r * a.cos(), r * a.sin()], r_ref, escala, centro)
        })
        .collect();
    for i in 0..N {
        if !tracejado || i % 4 < 2 {
            p.line_segment([pts[i], pts[i + 1]], stroke);
        }
    }
}

/// Esquema do perfil: três dentes, as circunferências da, d, db (quando acima da raiz) e df, e o
/// passo e a espessura medidos sobre o primitivo. `destaque` engrossa a grandeza sob o mouse no
/// rótulo do cálculo. Só existe para engrenagem externa.
pub fn esquema(ui: &mut egui::Ui, p: &ParamEngrenagem, c: &CalculoEngrenagem) {
    let destaque = ui.ctx().data(|d| d.get_temp::<Grandeza>(id_hover()));
    // a grandeza vale só enquanto o mouse está sobre a grade; o próximo quadro a refaz
    ui.ctx().data_mut(|d| d.remove::<Grandeza>(id_hover()));
    let Ok(pts) = fenkra_solidos::engrenagens::contorno_pontos(p, 8, 3) else {
        ui.label(RichText::new(tr("O esquema do dente vale para engrenagem externa.")).weak().small());
        return;
    };
    let (w, h) = (ui.available_width().min(380.), 190.);
    let (resp, painter) = ui.allocate_painter(egui::vec2(w, h), egui::Sense::hover());
    let v = ui.visuals().clone();
    let (texto, suave, acento) = (v.text_color(), v.weak_text_color(), crate::theme::accent(&v));
    painter.rect_filled(resp.rect, 4., v.extreme_bg_color);
    let (ra, r, rb, rf) = (c.diametro_externo / 2., c.diametro_primitivo / 2., c.diametro_base / 2., c.diametro_raiz / 2.);
    let passo_ang = std::f64::consts::TAU / p.dentes.max(1) as f64;
    // janela: altura do dente + folga; escala para caber três dentes em largura
    let altura = (ra - rf).max(1e-6);
    let escala = (f64::from(h) * 0.62 / altura).min(f64::from(w) * 0.9 / (3. * passo_ang * r));
    let r_ref = (ra + rf) / 2.;
    let centro = resp.rect.center();
    let limite = 1.5 * passo_ang;
    // dentes vizinhos em volta do dente 0 (ângulo em (-limite, limite))
    let dentro: Vec<Pos2> = pts
        .iter()
        .filter(|q| {
            let a = q[1].atan2(q[0]);
            a.abs() <= limite
        })
        .map(|q| ponto_do_esquema(*q, r_ref, escala, centro))
        .collect();
    painter.with_clip_rect(resp.rect.shrink(2.)).add(egui::Shape::line(dentro, Stroke::new(1.6, texto)));
    let est = |g: Grandeza, base: f32| if destaque == Some(g) { Stroke::new(base + 2.2, acento) } else { Stroke::new(base, suave) };
    let clip = painter.with_clip_rect(resp.rect.shrink(2.));
    arco_no_esquema(&clip, ra, limite, r_ref, escala, centro, est(Grandeza::Topo, 1.), true);
    arco_no_esquema(&clip, r, limite, r_ref, escala, centro, est(Grandeza::Primitivo, 1.), true);
    if rb > rf {
        arco_no_esquema(&clip, rb, limite, r_ref, escala, centro, est(Grandeza::Base, 1.), true);
    }
    arco_no_esquema(&clip, rf, limite, r_ref, escala, centro, est(Grandeza::Raiz, 1.), true);
    // rótulos das circunferências, à esquerda
    let esq = resp.rect.left() + 6.;
    for (rr, nome, gr) in [(ra, "da", Grandeza::Topo), (r, "d", Grandeza::Primitivo), (rb, "db", Grandeza::Base), (rf, "df", Grandeza::Raiz)] {
        if gr == Grandeza::Base && rb <= rf {
            continue;
        }
        let y = ponto_do_esquema([rr, 0.], r_ref, escala, centro).y;
        let cor = if destaque == Some(gr) { acento } else { texto };
        painter.text(Pos2::new(esq, y - 2.), egui::Align2::LEFT_BOTTOM, nome, egui::FontId::proportional(12.), cor);
    }
    // espessura do dente (corda no primitivo) e passo (do centro do dente 0 ao do dente 1)
    let semi = c.espessura_dente / (2. * r);
    let a_esp = ponto_do_esquema([r * semi.cos(), -r * semi.sin()], r_ref, escala, centro);
    let b_esp = ponto_do_esquema([r * semi.cos(), r * semi.sin()], r_ref, escala, centro);
    let st_esp = est(Grandeza::Espessura, 1.5);
    let st_esp = if destaque == Some(Grandeza::Espessura) { st_esp } else { Stroke::new(1.5, acento.gamma_multiply(0.7)) };
    painter.line_segment([a_esp, b_esp], st_esp);
    painter.text(a_esp.lerp(b_esp, 0.5) + Vec2::new(0., -4.), egui::Align2::CENTER_BOTTOM, "s", egui::FontId::proportional(12.), texto);
    let (p0, p1) = (ponto_do_esquema([r, 0.], r_ref, escala, centro), ponto_do_esquema([r * passo_ang.cos(), r * passo_ang.sin()], r_ref, escala, centro));
    let y_passo = p0.y + 14.;
    let st_passo = if destaque == Some(Grandeza::Passo) { Stroke::new(3.5, acento) } else { Stroke::new(1.5, acento.gamma_multiply(0.7)) };
    painter.line_segment([Pos2::new(p0.x, y_passo), Pos2::new(p1.x, y_passo)], st_passo);
    painter.text(Pos2::new((p0.x + p1.x) / 2., y_passo + 1.), egui::Align2::CENTER_TOP, "p", egui::FontId::proportional(12.), texto);
    // altura do dente
    let (ya, yf) = (ponto_do_esquema([ra, 0.], r_ref, escala, centro).y, ponto_do_esquema([rf, 0.], r_ref, escala, centro).y);
    let xh = resp.rect.right() - 14.;
    let st_h = if destaque == Some(Grandeza::Altura) { Stroke::new(3.5, acento) } else { Stroke::new(1.5, acento.gamma_multiply(0.7)) };
    painter.line_segment([Pos2::new(xh, ya), Pos2::new(xh, yf)], st_h);
    painter.text(Pos2::new(xh - 4., (ya + yf) / 2.), egui::Align2::RIGHT_CENTER, "h", egui::FontId::proportional(12.), texto);
}

// ---------------------------------------------------------------- modo Dimensionar

/// Seletor de modo e, no modo Dimensionar, os campos a e i com as sugestões. Aplicar uma sugestão
/// escreve módulo e dentes nos campos de projeto. Devolve se algo foi aplicado.
pub fn painel_dimensionar(ui: &mut egui::Ui, g: &mut GearsUi) -> bool {
    let mut aplicou = false;
    ui.horizontal(|ui| {
        ui.selectable_value(&mut g.dimensionar, false, tr("Projetar por módulo e dentes")).on_hover_text(tr("Você escolhe o módulo e os dentes; a distância entre centros e a razão saem do cálculo."));
        ui.selectable_value(&mut g.dimensionar, true, tr("Dimensionar por centros e razão")).on_hover_text(tr("Você diz a distância entre centros e a razão; o módulo e os dentes saem sugeridos."));
    });
    if !g.dimensionar || !matches!(g.p.tipo, TipoEngrenagem::Reta | TipoEngrenagem::Helicoidal) {
        return false;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Distância entre centros a"));
        ui.add(crate::ui_ext::dv(&mut g.dim_a).range(1.0..=5000.).speed(0.5).suffix(" mm").dica(tr("Distância entre os eixos das duas engrenagens: a = m (z1 + z2) / 2.")));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Razão i = z2 / z1"));
        ui.add(crate::ui_ext::dv(&mut g.dim_i).range(1.0..=20.).speed(0.02).max_decimals(3).dica(tr("Quantas voltas do pinhão para uma da coroa. Use 1 ou mais: o pinhão é a menor.")));
    });
    let s = sugerir_par(g.dim_a, g.dim_i, g.p.angulo_pressao);
    if s.is_empty() {
        ui.label(RichText::new(tr("Sem sugestão para esses valores.")).weak().small());
    }
    for x in &s {
        let erro = if x.erro_distancia.abs() < 1e-9 && x.erro_razao.abs() < 1e-9 { tr("exato").to_string() } else { format!("Δa {} · Δi {}", fmt(x.erro_distancia), fmt(x.erro_razao)) };
        let texto = format!("m {} · z1 {} · z2 {} · a {} · i {} · {erro}", fmt(x.modulo), x.z1, x.z2, fmt(x.distancia), fmt(x.razao));
        if ui.button(texto).on_hover_text(tr("Aplica módulo e dentes ao projeto.")).clicked() {
            g.p.modulo = x.modulo;
            g.p.dentes = x.z1;
            g.dentes_coroa = x.z2;
            aplicou = true;
        }
    }
    aplicou
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotulos_do_calculo_viram_grandezas() {
        assert_eq!(Grandeza::do_rotulo("Diâmetro de topo da"), Some(Grandeza::Topo));
        assert_eq!(Grandeza::do_rotulo("Passo circular"), Some(Grandeza::Passo));
        assert_eq!(Grandeza::do_rotulo("Dentes da cremalheira"), None);
    }

    #[test]
    fn a_largura_da_alca_para_em_meio_milimetro() {
        assert_eq!(largura_arredondada(10.24), 10.);
        assert_eq!(largura_arredondada(10.26), 10.5);
        assert_eq!(largura_arredondada(-5.), 0.5);
        assert_eq!(largura_arredondada(1e9), 1000.);
    }

    #[test]
    fn o_esquema_poe_o_raio_para_cima_e_o_dente_zero_no_meio() {
        let c = Pos2::new(100., 100.);
        let topo = ponto_do_esquema([22., 0.], 20., 10., c);
        assert!((topo.x - 100.).abs() < 1e-4 && (topo.y - 80.).abs() < 1e-4);
        let lado = ponto_do_esquema([20., 1.5], 20., 10., c);
        assert!((lado.x - 115.).abs() < 1e-4 && (lado.y - 100.).abs() < 1e-4);
    }

    #[test]
    fn a_previa_poe_o_par_na_distancia_e_na_razao_do_calculo() {
        // 20 e 40 dentes, m2: centros a 60 e razão -1/2; as duas malhas só dependem dos parâmetros
        let g = GearsUi::default();
        let pv: PreviaPar = previa_par(&ParamEngrenagem { dentes: 20, ..g.p.clone() }, 40).unwrap();
        assert!((pv.distancia_medida() - 60.).abs() < 1e-9 && (pv.par.razao + 0.5).abs() < 1e-12);
    }

    #[test]
    fn dimensionar_aplica_modulo_e_dentes_da_sugestao_e_a_previa_confere() {
        let mut g = GearsUi::default();
        g.dim_a = 90.;
        g.dim_i = 2.;
        let s = sugerir_par(g.dim_a, g.dim_i, g.p.angulo_pressao);
        let x = s.iter().find(|x| x.modulo == 3.).expect("m3");
        g.p.modulo = x.modulo;
        g.p.dentes = x.z1;
        g.dentes_coroa = x.z2;
        let pv = previa_par(&g.p, g.dentes_coroa).unwrap();
        assert!((pv.distancia_medida() - 90.).abs() < 1e-9, "{}", pv.distancia_medida());
        assert!((pv.par.razao + 0.5).abs() < 1e-12, "z1/z2 = 20/40");
    }
}
