// SPDX-License-Identifier: LGPL-2.1-or-later
//! Limites de junta na interface (B36). O painel mostra Mínimo, Atual e Máximo de cada grau de
//! liberdade, com uma caixa por extremo; o 3D mostra o arco ou o trilho com as marcas arrastáveis. O
//! solucionador é quem impõe a faixa: aqui só se escreve nela.

use crate::app::{fmt, App};
use crate::i18n::tr;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use fenkra_core::Id;
use fenkra_montagem::kin::{self, Faixa, Grau, Joint, Limite, Limites};
use fenkra_montagem::simbolos::{marcas_limite, MarcaLimite};
use fenkra_montagem::Assembly;
use glam::Vec3;
use std::collections::HashMap;

/// Raio do arco de limites na tela (px).
const RAIO: f64 = 64.;

/// Passos em que o arco é amostrado para achar o ângulo sob o mouse (graus).
const AMOSTRAS_ARCO: i32 = 360;

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// O que a pessoa fez no painel.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Pedido {
    /// digitou no campo Atual: leva a junta a este valor
    pub levar: Option<(Grau, f64)>,
}

fn extremo(ui: &mut egui::Ui, nome: &str, dica: &str, l: &mut Limite, unidade: &'static str, inicial: f64) {
    let mut on = l.ligado;
    if ui.checkbox(&mut on, tr(nome)).on_hover_text(tr("Liga este limite; desligar guarda o valor")).changed() {
        // valor nunca usado nasce no padrão do grau; o que já foi digitado volta como estava
        if on && l.valor == 0. {
            l.valor = inicial;
        }
        l.ligado = on;
    }
    ui.add_enabled_ui(l.ligado, |ui| {
        ui.add(crate::ui_ext::dv(&mut l.valor).speed(1.).suffix(unidade).dica(dica));
    });
    ui.end_row();
}

/// Mínimo, Atual e Máximo de cada grau. `atual` é a medida de agora; com `editavel` o campo Atual
/// move a junta.
pub fn painel(ui: &mut egui::Ui, salt: &str, graus: &[Grau], lim: &mut Limites, atual: impl Fn(Grau) -> Option<f64>, editavel: impl Fn(Grau) -> bool) -> Pedido {
    let mut pedido = Pedido::default();
    if graus.is_empty() {
        ui.label(RichText::new(tr("Esta junta não tem curso nem ângulo para limitar.")).weak().small());
        return pedido;
    }
    for g in graus {
        ui.label(RichText::new(tr(g.label())).strong());
        egui::Grid::new((salt, g.label())).num_columns(2).spacing([8., 2.]).show(ui, |ui| {
            let f = lim.faixa_mut(*g);
            let (lo, hi) = g.padrao();
            extremo(ui, "Mínimo", "Até onde a junta vai para o lado negativo", &mut f.min, g.unidade(), lo);
            ui.label(tr("Atual"));
            match atual(*g) {
                Some(mut v) if editavel(*g) => {
                    if ui.add(crate::ui_ext::dv(&mut v).speed(1.).suffix(g.unidade()).dica("Leva a junta a este valor")).changed() {
                        pedido.levar = Some((*g, v));
                    }
                }
                Some(v) => {
                    ui.label(format!("{}{}", fmt(v), g.unidade()));
                }
                None => {
                    ui.label(RichText::new("—").weak());
                }
            }
            ui.end_row();
            extremo(ui, "Máximo", "Até onde a junta vai para o lado positivo", &mut f.max, g.unidade(), hi);
        });
        let f = lim.faixa(*g);
        if f.min.ligado && f.max.ligado && f.min.valor > f.max.valor {
            ui.label(RichText::new(tr("O mínimo passou do máximo: a junta fica sem solução.")).small().color(crate::theme::st_err()));
        }
    }
    pedido
}

/// Texto curto da faixa, para a dica da linha na árvore.
pub fn resumo(j: &Joint) -> String {
    let lado = |l: &Limite, u: &str| if l.ligado { format!("{}{u}", fmt(l.valor)) } else { "…".to_string() };
    let partes: Vec<String> = j.kind.graus().iter().filter(|g| j.limites.faixa(**g).ligada()).map(|g| {
        let f = j.limites.faixa(*g);
        format!("{}: {} a {}", tr(g.label()), lado(&f.min, g.unidade()), lado(&f.max, g.unidade()))
    }).collect();
    partes.join(" · ")
}

/// A junta tem algum limite ligado?
pub fn tem_limite(j: &Joint) -> bool {
    j.kind.graus().iter().any(|g| j.limites.faixa(*g).ligada())
}

/// Valor no trilho ou no arco sob o mouse.
fn valor_sob(m: &MarcaLimite, raio: f64, rect: Rect, cam: &Camera, mouse: Pos2) -> Option<f64> {
    if m.grau.angular() {
        let perto = (-AMOSTRAS_ARCO..=AMOSTRAS_ARCO).filter_map(|g| cam.project(rect, v3(m.ponto(g as f64, raio))).map(|q| ((q - mouse).length(), g as f64)));
        return perto.min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1);
    }
    let (_, na_tela, ppm) = crate::gizmo::screen_axis(cam, rect, v3(m.ponto(0., 0.)), v3(m.eixo))?;
    let zero = cam.project(rect, v3(m.ponto(0., 0.)))?;
    Some(((mouse - zero).dot(na_tela) / ppm) as f64)
}

/// Pontos de `de` até `ate`, para desenhar o trecho.
fn linha(m: &MarcaLimite, raio: f64, de: f64, ate: f64, rect: Rect, cam: &Camera) -> Vec<Pos2> {
    let n = if m.grau.angular() { 48 } else { 1 };
    (0..=n).filter_map(|k| cam.project(rect, v3(m.ponto(de + (ate - de) * k as f64 / n as f64, raio)))).collect()
}

/// Extremos da faixa desenhada: quem está desligado termina na medida de agora.
fn trecho(m: &MarcaLimite) -> (f64, f64) {
    m.faixa.ou((m.atual.min(m.faixa.max.valor), m.atual.max(m.faixa.min.valor)))
}

/// Uma marca arrastada: o grau, se é o máximo, o valor novo e se o arrasto acabou.
struct Arrasto {
    grau: Grau,
    maximo: bool,
    valor: f64,
    fim: bool,
}

/// Arco ou trilho com as marcas. Devolve a marca que estiver em arrasto.
fn desenhar(ui: &mut egui::Ui, rect: Rect, cam: &Camera, marcas: &[MarcaLimite], chave: u64) -> Option<Arrasto> {
    let v = ui.visuals().clone();
    let cor = crate::theme::accent(&v);
    let mut out = None;
    for m in marcas {
        let Some((_, _, ppm)) = crate::gizmo::screen_axis(cam, rect, v3(m.ponto(0., 0.)), v3(m.eixo)) else { continue };
        let raio = RAIO / (ppm as f64).max(1e-3);
        let (lo, hi) = trecho(m);
        let p = ui.painter_at(rect);
        p.add(egui::Shape::line(linha(m, raio, lo, hi, rect, cam), Stroke::new(2.4, cor.gamma_multiply(0.75))));
        if let Some(a) = cam.project(rect, v3(m.ponto(m.atual, raio))) {
            p.circle_stroke(a, 4., Stroke::new(1.6, v.text_color()));
        }
        for (maximo, l) in [(false, m.faixa.min), (true, m.faixa.max)] {
            if !l.ligado {
                continue;
            }
            let Some(em) = cam.project(rect, v3(m.ponto(l.valor, raio))) else { continue };
            if let Some(a) = arrastavel(ui, rect, cam, m, raio, (chave, maximo, m.grau as u8), em, cor, l.valor) {
                out = Some(Arrasto { grau: m.grau, maximo, valor: a.0, fim: a.1 });
            }
        }
    }
    out
}

/// Marca de um extremo, com a pílula do valor. Devolve o valor em arrasto e se ele acabou.
#[allow(clippy::too_many_arguments)]
fn arrastavel(ui: &mut egui::Ui, rect: Rect, cam: &Camera, m: &MarcaLimite, raio: f64, chave: (u64, bool, u8), em: Pos2, cor: Color32, valor: f64) -> Option<(f64, bool)> {
    let r = ui.interact(Rect::from_center_size(em, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(("asm_limite", chave)), egui::Sense::drag()).on_hover_text(tr("Arraste para mudar o limite"));
    let quente = r.hovered() || r.dragged();
    let p = ui.painter_at(rect);
    p.circle_filled(em, if quente { 6.5 } else { 4.5 }, cor);
    let texto = format!("{}{}", fmt(valor), m.grau.unidade());
    p.text(em + egui::vec2(9., -9.), egui::Align2::LEFT_BOTTOM, texto, egui::FontId::proportional(12.), crate::theme::readable(cor, ui.visuals()));
    if !(r.dragged() || r.drag_stopped()) {
        return None;
    }
    let mouse = ui.input(|i| i.pointer.interact_pos())?;
    let bruto = valor_sob(m, raio, rect, cam, mouse)?;
    let passo = if m.grau.angular() { ui.input(|i| crate::ui_ext::mult_teclas(i.modifiers)) } else { crate::gizmo::screen_axis(cam, rect, v3(m.ponto(0., 0.)), v3(m.eixo)).map_or(1., |t| crate::ui_ext::passo_3d(ui, t.2)) };
    Some(((bruto / passo).round() * passo, r.drag_stopped()))
}

fn aplicar(f: &mut Faixa, maximo: bool, v: f64) {
    // uma marca não passa da outra: a faixa nunca fica invertida pelo arrasto
    if maximo {
        f.max.valor = if f.min.ligado { v.max(f.min.valor) } else { v };
    } else {
        f.min.valor = if f.max.ligado { v.min(f.max.valor) } else { v };
    }
}

/// Marcas da junta que tem limite ligado.
fn da_junta(a: &Assembly, id: Id) -> Vec<MarcaLimite> {
    a.joints.iter().find(|j| j.id == id && !j.suppressed).map(|j| marcas_limite(a, j)).unwrap_or_default()
}

/// Marcas no 3D: da junta escolhida na árvore ou da que está sendo criada.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(st) = app.asm.as_mut() else { return };
    if st.editing_part.is_some() {
        return;
    }
    if let Some(pen) = st.cmd.as_mut() {
        let Some((copia, id)) = pen.previa().and_then(|p| Some((&p.copia, p.id?))) else { return };
        let marcas = da_junta(copia, id);
        if let Some(a) = desenhar(ui, rect, cam, &marcas, id.0) {
            aplicar(pen.c.limites.faixa_mut(a.grau), a.maximo, a.valor);
            crate::asm_cmd::atualizar(st);
        }
        return;
    }
    let Some(id) = st.sel else { return };
    let marcas = da_junta(&st.asm, id);
    let Some(a) = desenhar(ui, rect, cam, &marcas, id.0) else { return };
    if ui.input(|i| i.pointer.any_pressed()) || !st.limite_em_arrasto {
        st.limite_em_arrasto = true;
        st.push_undo();
    }
    if let Some(j) = st.asm.joints.iter_mut().find(|j| j.id == id) {
        aplicar(j.limites.faixa_mut(a.grau), a.maximo, a.valor);
    }
    if a.fim {
        st.limite_em_arrasto = false;
        st.solve();
    } else {
        // no arrasto só as posições; o resto (links, GDL) espera soltar
        st.report = fenkra_montagem::solve::solve(&mut st.asm);
        st.dirty = true;
    }
}

/// Medida de cada grau da junta nas posições de agora.
pub fn medidor<'a>(j: &'a Joint, pl: &'a HashMap<Id, fenkra_montagem::Placement>) -> impl Fn(Grau) -> Option<f64> + 'a {
    move |g| Some(kin::medir_grau(j, g, pl.get(&j.a.comp)?, pl.get(&j.b.comp)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caixa_desligada_guarda_o_valor() {
        let mut f = Faixa::nova(-30., 45.);
        f.max.ligado = false;
        assert_eq!(f.max.valor, 45.);
        assert_eq!(f.prender(100.), 100.);
        f.max.ligado = true;
        assert_eq!(f.prender(100.), 45.);
    }

    #[test]
    fn marca_nao_passa_da_outra() {
        let mut f = Faixa::nova(-30., 45.);
        aplicar(&mut f, true, -60.);
        assert_eq!(f.max.valor, -30.);
        aplicar(&mut f, false, 10.);
        assert_eq!(f.min.valor, -30.);
        f.max.ligado = false;
        aplicar(&mut f, false, 10.);
        assert_eq!(f.min.valor, 10.);
    }

    #[test]
    fn todo_texto_do_painel_tem_traducao() {
        for t in ["Atual", "Mínimo", "Máximo", "Liga este limite; desligar guarda o valor", "Leva a junta a este valor", "Arraste para mudar o limite", "O mínimo passou do máximo: a junta fica sem solução.", "Até onde a junta vai para o lado negativo", "Até onde a junta vai para o lado positivo", "Parada no limite"] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
        for g in [Grau::Giro, Grau::Curso, Grau::CursoX, Grau::CursoY, Grau::Inclinacao, Grau::Trajeto] {
            assert!(crate::i18n::tem_traducao(g.label()), "{}", g.label());
        }
    }
}
