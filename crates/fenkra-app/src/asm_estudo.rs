// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estudo de movimento (B121): painel do comando na coluna da esquerda.
//!
//! O painel começa pelo que dá resposta na hora: um cursor por grau de liberdade de cada junta, que
//! leva o mecanismo pelo solucionador sem calcular quadros. Depois vêm as posições nomeadas (B123) e
//! o tempo: leis em `t`, quadros calculados, reprodução e cursor de tempo. Os gráficos do ponto
//! acompanhado têm eixos com marcas, unidades e o cursor do tempo. No 3D ficam a trajetória do ponto
//! e a lista do que ainda falta comandar, preso a cada junta.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use eframe::egui::{self, Color32, RichText};
use fenkra_core::Id;
use fenkra_montagem::estudo::{falta_comandar, FaltaComandar};
use fenkra_montagem::kin::{self, Grau};
use fenkra_montagem::NodeKind;

/// O que o estudo guarda entre um quadro e outro.
#[derive(Default)]
pub struct Estudo {
    /// ponto acompanhado, no sistema do componente (mm)
    pub ponto: [f64; 3],
    /// análise do mecanismo e do que falta, com a chave da montagem de quando foi feita
    analise: Option<(String, (usize, usize), FaltaComandar)>,
}

/// Chave barata do que muda a conta de graus de liberdade: juntas, acoplamentos, restrições e o que
/// está fixo. Posição não entra: o posto do sistema só muda em posição singular.
fn chave(st: &AsmState) -> String {
    let fixos: Vec<(Id, bool)> = st.asm.nodes.iter().map(|n| (n.id, n.fixed)).collect();
    format!("{:?}{:?}{:?}{:?}", st.asm.joints, st.asm.couplings, st.asm.mates, fixos)
}

/// Graus livres sem e com os comandos, e o que falta comandar; refeito só quando a montagem muda.
pub fn analise(st: &mut AsmState) -> ((usize, usize), FaltaComandar) {
    let k = chave(st);
    if st.estudo.analise.as_ref().is_none_or(|(c, _, _)| *c != k) {
        let gdl = kin::analyze(&st.asm);
        let falta = falta_comandar(&st.asm);
        st.estudo.analise = Some((k, gdl, falta));
    }
    let (_, gdl, falta) = st.estudo.analise.clone().unwrap();
    (gdl, falta)
}

/// Nome do grau que falta, para a lista: "Revolução 1 · ângulo".
fn rotulo_falta(st: &AsmState, j: Id, g: Grau) -> String {
    let nome = st.asm.joints.iter().find(|x| x.id == j).map(|x| x.name.clone()).unwrap_or_default();
    format!("{nome} · {}", tr(g.label()).to_lowercase())
}

/// O painel do estudo, na coluna da esquerda.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let Some(st) = app.asm.as_mut() else { return };
    if crate::app::done_bar(ui) {
        fechar(st);
        return;
    }
    let ((sem, com), falta) = analise(st);
    ui.label(RichText::new(format!("{} {sem} {} · {com} {}", tr("Mecanismo:"), tr("GDL sem comandos"), tr("com os comandos"))).strong());
    if com == 0 {
        ui.label(RichText::new(tr("✔ cada grau de liberdade tem comando: pode simular")).color(crate::theme::st_ok()));
    } else {
        ui.label(RichText::new(tr("Falta comandar (também marcado no 3D):")).color(crate::theme::st_warn()));
        for (j, g) in &falta.graus {
            ui.label(RichText::new(format!("  • {}", rotulo_falta(st, *j, *g))).small());
        }
        if falta.outros > 0 {
            ui.label(RichText::new(format!("  • {} {}", falta.outros, tr("GDL de peças sem junta ou de graus sem comando direto"))).small());
        }
    }
    crate::transmissao_ui::cadeia_na_simulacao(ui, st);
    cursores(st, ui);
    crate::asm_posicoes::secao(st, ui);
    tempo(st, ui);
    acompanhar(st, ui);
}

/// Cursor direto por grau de liberdade: mexe no mecanismo pelo solucionador, sem quadros.
fn cursores(st: &mut AsmState, ui: &mut egui::Ui) {
    crate::editor::section(ui, "Juntas");
    let pl: std::collections::HashMap<Id, fenkra_montagem::Placement> = st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect();
    let mut pedido = None;
    let mut comecou = false;
    for j in st.asm.joints.iter().filter(|j| !j.suppressed) {
        let (Some(pa), Some(pb)) = (pl.get(&j.a.comp), pl.get(&j.b.comp)) else { continue };
        for g in j.kind.graus().iter().copied().filter(|g| *g != Grau::Inclinacao) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} · {}", j.name, tr(g.label()).to_lowercase())).small());
                if !kin::aceita_levar(j, g) {
                    let lei = if g == Grau::Giro { j.drive_angle.clone() } else { j.drive_len.clone() };
                    ui.label(RichText::new(format!("{} {}", tr("comando:"), lei.unwrap_or_default())).weak().small()).on_hover_text(tr("O comando da junta manda neste grau. Desligue-o em Tempo para usar o cursor."));
                    return;
                }
                let faixa = j.limites.faixa(g);
                let (lo, hi) = if g == Grau::Giro && !faixa.ligada() { (-180., 180.) } else { faixa.ou(g.padrao()) };
                let mut v = kin::medir_grau(j, g, pa, pb);
                let r = ui.add(crate::ui_ext::slider(&mut v, lo..=hi).suffix(g.unidade())).on_hover_text(tr("Arraste: o mecanismo anda na hora, pelo solucionador, sem calcular quadros"));
                comecou |= r.drag_started();
                if r.changed() {
                    pedido = Some((j.id, g, v));
                }
            });
        }
    }
    if comecou {
        st.push_undo();
    }
    if let Some((id, g, v)) = pedido {
        // o cursor manda: a reprodução para e os quadros calculados saem, senão eles repõem a posição
        st.sim_play = false;
        st.sim_frames.clear();
        st.sim_rest = None;
        st.report = kin::levar_a(&mut st.asm, id, g, v);
        st.dirty = true;
    }
}

/// Leis em `t`, cálculo dos quadros e reprodução.
fn tempo(st: &mut AsmState, ui: &mut egui::Ui) {
    crate::editor::section(ui, "Tempo");
    ui.label(RichText::new(tr("Leis no tempo t, em segundos. O cálculo dos quadros só serve para tocar no tempo.")).weak().small());
    for j in st.asm.joints.iter_mut().filter(|j| !j.suppressed) {
        let (ca, cl) = j.kind.drives();
        if !(ca || cl) {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(&j.name).small());
            if ca {
                let mut on = j.drive_angle.is_some();
                if ui.checkbox(&mut on, "°").on_hover_text(tr("Lei do ângulo no tempo, em graus")).changed() {
                    j.drive_angle = on.then(|| "90*t".to_string());
                }
                if let Some(e) = &mut j.drive_angle {
                    ui.add(egui::TextEdit::singleline(e).desired_width(90.));
                }
            }
            if cl {
                let mut on = j.drive_len.is_some();
                if ui.checkbox(&mut on, "mm").on_hover_text(tr("Lei do curso no tempo, em mm")).changed() {
                    j.drive_len = on.then(|| "10*t".to_string());
                }
                if let Some(e) = &mut j.drive_len {
                    ui.add(egui::TextEdit::singleline(e).desired_width(90.));
                }
            }
        });
    }
    let mut calcular = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Duração"));
        let r = ui.add(crate::ui_ext::dv(&mut st.sim_t1).range(0.01..=600.0).suffix(" s").speed(0.1).dica(tr("Tempo final da simulação")));
        crate::ui_ext::wheel(ui, &r, &mut st.sim_t1);
        ui.label(tr("Quadros"));
        let r = ui.add(crate::ui_ext::dv(&mut st.sim_steps).range(2..=2000).dica(tr("Quantos quadros entre zero e a duração")));
        crate::ui_ext::wheel(ui, &r, &mut st.sim_steps);
        if ui.button(tr("⏺ Calcular quadros")).clicked() {
            calcular = true;
        }
    });
    if calcular {
        if st.sim_rest.is_none() {
            st.sim_rest = Some(st.asm.nodes.iter().map(|n| (n.id, n.placement)).collect());
        }
        let (t1, n) = (st.sim_t1, st.sim_steps);
        st.sim_frames = kin::simulate(&mut st.asm, 0., t1, n);
        st.sim_t = 0.;
        st.sim_play = true;
    }
    if st.sim_frames.is_empty() {
        ui.label(RichText::new(tr("Defina os comandos e calcule os quadros; depois toque ou arraste o tempo.")).weak().small());
        return;
    }
    let falhas = st.sim_frames.iter().filter(|f| !f.ok).count();
    let extra = if falhas > 0 { format!(" · {falhas} {}", tr("sem solução (trava ou fim de curso)")) } else { String::new() };
    ui.label(RichText::new(format!("{} {}{extra}", st.sim_frames.len(), tr("quadros"))).small());
    let mut repor = false;
    ui.horizontal(|ui| {
        if ui.button(if st.sim_play { "⏸" } else { "▶" }).on_hover_text(tr("Tocar ou pausar")).clicked() {
            st.sim_play = !st.sim_play;
        }
        if ui.button("⏮").on_hover_text(tr("Voltar ao começo")).clicked() {
            st.sim_t = 0.;
        }
        let t1 = st.sim_t1;
        if ui.add(crate::ui_ext::slider(&mut st.sim_t, 0.0..=t1).text("t (s)")).on_hover_text(tr("Cursor de tempo: arraste para percorrer os quadros")).changed() {
            st.sim_play = false;
        }
        if ui.button(tr("Repor")).on_hover_text(tr("volta às posições de antes da simulação")).clicked() {
            repor = true;
        }
    });
    if repor {
        repor_posicoes(st);
    }
}

/// Ponto acompanhado e os gráficos dele no tempo.
fn acompanhar(st: &mut AsmState, ui: &mut egui::Ui) {
    crate::editor::section(ui, "Ponto acompanhado");
    let comps: Vec<(Id, String)> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| (n.id, n.name.clone())).collect();
    let nome = st.sim_track.map(|c| st.asm.name_of(c)).unwrap_or_else(|| tr("acompanhar componente…").into());
    egui::ComboBox::from_id_salt("sim_track").selected_text(nome).show_ui(ui, |ui| {
        for (id, n) in &comps {
            ui.selectable_value(&mut st.sim_track, Some(*id), n);
        }
    });
    let Some(c) = st.sim_track else { return };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Ponto (mm, no componente)")).small());
        for (k, eixo) in ["X", "Y", "Z"].iter().enumerate() {
            ui.add(crate::ui_ext::dv(&mut st.estudo.ponto[k]).speed(0.5).prefix(format!("{eixo} ")).max_decimals(2).dica(tr("Ponto do componente cuja trajetória, velocidade e aceleração aparecem")));
        }
    });
    if st.sim_frames.is_empty() {
        ui.label(RichText::new(tr("Calcule os quadros ou toque as posições para ver a trajetória e os gráficos.")).weak().small());
        return;
    }
    let k = kin::kinematics_of(&st.sim_frames, c, st.estudo.ponto);
    let Some(p0) = k.first().map(|x| x.1) else { return };
    let desloc: Vec<(f64, f64)> = k.iter().map(|x| (x.0, fenkra_montagem::norm(fenkra_montagem::sub(x.1, p0)))).collect();
    let vel: Vec<(f64, f64)> = k.iter().map(|x| (x.0, x.2)).collect();
    let acel: Vec<(f64, f64)> = k.iter().map(|x| (x.0, x.3)).collect();
    let (t1, t) = (st.sim_t1, st.sim_t);
    let acento = crate::theme::accent(ui.visuals());
    grafico(ui, tr("Distância ao início"), "mm", &desloc, t1, t, acento);
    grafico(ui, tr("Velocidade"), "mm/s", &vel, t1, t, acento);
    grafico(ui, tr("Aceleração"), "mm/s²", &acel, t1, t, crate::theme::st_warn());
}

/// Marcas "redondas" de um eixo de `lo` a `hi`: passo 1, 2 ou 5 vezes uma potência de dez, com perto
/// de `alvo` marcas. Faixa vazia ganha uma faixa em volta do valor.
pub fn marcas(lo: f64, hi: f64, alvo: usize) -> Vec<f64> {
    let (lo, hi) = if (hi - lo).abs() < 1e-12 { (lo - 1., hi + 1.) } else { (lo.min(hi), lo.max(hi)) };
    let bruto = (hi - lo) / alvo.max(1) as f64;
    let dez = 10f64.powf(bruto.log10().floor());
    let passo = [1., 2., 5., 10.].iter().map(|m| m * dez).find(|p| *p >= bruto * 0.999).unwrap_or(10. * dez);
    let mut v = vec![];
    let mut x = (lo / passo).ceil() * passo;
    while x <= hi + passo * 1e-9 {
        v.push(if x.abs() < passo * 1e-9 { 0. } else { x });
        x += passo;
    }
    v
}

/// Gráfico de uma grandeza no tempo, com eixos, marcas, unidade e o cursor de tempo.
fn grafico(ui: &mut egui::Ui, titulo: &str, unidade: &str, pts: &[(f64, f64)], t1: f64, t: f64, cor: Color32) {
    let agora = pts.iter().min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs())).map(|p| p.1);
    ui.label(RichText::new(format!("{titulo} ({unidade}){}", agora.map(|v| format!(" · {} {unidade} {} t = {} s", fmt(v), tr("em"), fmt(t))).unwrap_or_default())).small());
    // o que cabe à vista: a coluna corta o que passa da borda, e a barra de rolagem toma a direita
    let largura = ui.available_width().min(ui.clip_rect().right() - ui.cursor().left() - 10.).max(120.);
    let (r, _) = ui.allocate_exact_size(egui::vec2(largura, 104.), egui::Sense::hover());
    let p = ui.painter_at(r);
    let vis = ui.visuals();
    p.rect_filled(r, 4., vis.extreme_bg_color);
    let fonte = egui::FontId::proportional(10.);
    let texto = vis.weak_text_color();
    let grade = egui::Stroke::new(0.5, vis.widgets.noninteractive.bg_stroke.color);
    // área do traço: margem à esquerda para os valores e embaixo para o tempo
    let area = egui::Rect::from_min_max(r.min + egui::vec2(42., 6.), r.max - egui::vec2(8., 16.));
    let (lo, hi) = pts.iter().fold((0f64, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let ys = marcas(lo, hi.max(lo), 4);
    let (y0, y1) = (ys.first().copied().unwrap_or(lo).min(lo), ys.last().copied().unwrap_or(hi).max(hi));
    let tmax = t1.max(1e-9);
    let em = |tt: f64, v: f64| egui::pos2(area.left() + (tt / tmax).clamp(0., 1.) as f32 * area.width(), area.bottom() - ((v - y0) / (y1 - y0).max(1e-12)) as f32 * area.height());
    for y in &ys {
        let a = em(0., *y);
        p.line_segment([a, egui::pos2(area.right(), a.y)], grade);
        p.text(egui::pos2(area.left() - 4., a.y), egui::Align2::RIGHT_CENTER, fmt(*y), fonte.clone(), texto);
    }
    for x in marcas(0., tmax, 5) {
        let a = em(x, y0);
        p.line_segment([egui::pos2(a.x, area.top()), egui::pos2(a.x, area.bottom())], grade);
        // a marca mais perto da borda dá lugar ao nome do eixo
        if area.right() - a.x > 30. {
            p.text(egui::pos2(a.x, area.bottom() + 2.), egui::Align2::CENTER_TOP, fmt(x), fonte.clone(), texto);
        }
    }
    p.text(egui::pos2(area.right(), area.bottom() + 2.), egui::Align2::RIGHT_TOP, "t (s)", fonte.clone(), texto);
    let eixo = egui::Stroke::new(1., vis.text_color());
    p.line_segment([area.left_bottom(), area.right_bottom()], eixo);
    p.line_segment([area.left_bottom(), area.left_top()], eixo);
    p.add(egui::Shape::line(pts.iter().map(|q| em(q.0, q.1)).collect(), egui::Stroke::new(1.8, cor)));
    let xt = em(t, y0).x;
    p.line_segment([egui::pos2(xt, area.top()), egui::pos2(xt, area.bottom())], egui::Stroke::new(1., vis.text_color()));
    if let Some(v) = agora {
        p.circle_filled(em(t, v), 3., cor);
    }
}

/// Volta às posições de antes de calcular ou tocar, e tira os quadros.
pub fn repor_posicoes(st: &mut AsmState) {
    if let Some(rest) = st.sim_rest.take() {
        for (id, p) in rest {
            if let Some(n) = st.asm.node_mut(id) {
                n.placement = p;
            }
        }
        st.dirty = true;
    }
    st.sim_frames.clear();
    st.sim_play = false;
}

/// Fecha o estudo: devolve as posições de antes da reprodução.
pub fn fechar(st: &mut AsmState) {
    repor_posicoes(st);
    st.sim_open = false;
}

/// A cada quadro: exportação em andamento, reprodução e o quadro do tempo atual.
pub fn reproducao(app: &mut App, ctx: &egui::Context) {
    crate::asm_posicoes::passo(app, ctx);
    let Some(st) = app.asm.as_mut() else { return };
    demo(st);
    if !st.sim_open || st.sim_frames.is_empty() || st.posicoes_ui.exportando.is_some() {
        return;
    }
    if st.sim_play {
        st.sim_t += ctx.input(|i| i.stable_dt) as f64;
        if st.sim_t > st.sim_t1 {
            st.sim_t = 0.;
        }
        ctx.request_repaint();
    }
    let t = st.sim_t;
    let Some(f) = st.sim_frames.iter().min_by(|a, b| (a.t - t).abs().total_cmp(&(b.t - t).abs())).cloned() else { return };
    let mut mudou = false;
    for n in &mut st.asm.nodes {
        if let Some(p) = f.placements.get(&n.id) {
            if n.placement != *p {
                n.placement = *p;
                mudou = true;
            }
        }
    }
    st.dirty |= mudou;
}

/// Captura sem mouse, com `FENKRA_DEMO_ASM=mec`: `FENKRA_DEMO_ESTUDO=falta` abre o estudo com o giro
/// sem comando; `posicoes` grava três posições, toca e para no meio do segundo trecho, acompanhando
/// a placa de cima; `exportar` faz o mesmo e exporta para `FENKRA_DEMO_EXPORTAR_ANIMACAO`.
fn demo(st: &mut AsmState) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static FEITO: AtomicBool = AtomicBool::new(false);
    let Ok(modo) = std::env::var("FENKRA_DEMO_ESTUDO") else { return };
    if st.asm.joints.is_empty() || FEITO.swap(true, Ordering::Relaxed) {
        return;
    }
    let comps: Vec<Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| n.id).collect();
    if let Some(n) = comps.first().and_then(|c| st.asm.node_mut(*c)) {
        n.fixed = true;
    }
    let j = st.asm.joints[0].id;
    st.asm.joints[0].drive_angle = None;
    st.sim_open = true;
    st.sim_track = comps.get(1).copied();
    if modo == "falta" {
        st.solve();
        return;
    }
    for (giro, nome, t) in [(0., "Fechada", 0.), (90., "Meia", 2.), (180., "Aberta", 3.)] {
        kin::levar_a(&mut st.asm, j, Grau::Giro, giro);
        let i = st.asm.nova_posicao(nome);
        st.asm.posicoes[i].transicao = t;
    }
    let p = st.asm.posicoes[0].clone();
    fenkra_montagem::posicoes::aplicar(&mut st.asm, &p);
    crate::asm_posicoes::tocar(st);
    st.sim_play = false;
    st.sim_t = 3.5;
    st.dirty = true;
    if modo == "exportar" {
        if let Ok(pasta) = std::env::var("FENKRA_DEMO_EXPORTAR_ANIMACAO") {
            if let Err(e) = crate::asm_posicoes::exportar(st, pasta.into()) {
                eprintln!("exportar: {e}");
            }
        }
    }
}

/// No 3D: trajetória do ponto acompanhado e o que falta comandar, preso a cada junta.
pub fn no_3d(st: &mut AsmState, ui: &egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    if !st.sim_open {
        return;
    }
    let painter = ui.painter_at(rect);
    let v3 = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let vis = ui.visuals();
    let acento = crate::theme::accent(vis);
    // trajetória do ponto, com o lugar do tempo atual
    if let Some(c) = st.sim_track.filter(|_| !st.sim_frames.is_empty()) {
        let k = kin::kinematics_of(&st.sim_frames, c, st.estudo.ponto);
        let pts: Vec<egui::Pos2> = k.iter().filter_map(|x| cam.project(rect, v3(x.1))).collect();
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(2., acento)));
        }
        let t = st.sim_t;
        if let Some(agora) = k.iter().min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs())).and_then(|x| cam.project(rect, v3(x.1))) {
            painter.circle_filled(agora, 5., acento);
            painter.circle_stroke(agora, 5., egui::Stroke::new(1.5, vis.window_fill()));
        }
    }
    // o que falta comandar: etiqueta em cada junta e a lista no canto
    let ((_, com), falta) = analise(st);
    if com == 0 {
        return;
    }
    let aviso = crate::theme::st_warn();
    let fundo = vis.window_fill().gamma_multiply(0.92);
    let fonte = egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.));
    let mut linhas = vec![];
    for (j, g) in &falta.graus {
        let rotulo = rotulo_falta(st, *j, *g);
        if let Some(junta) = st.asm.joints.iter().find(|x| x.id == *j) {
            if let Some(pa) = st.asm.lugar(junta.a.comp) {
                if let Some(s) = cam.project(rect, v3(junta.a.frame.world(&pa).0)) {
                    let texto = format!("{} {}", if *g == Grau::Giro { "⟳" } else { "↔" }, rotulo);
                    let gal = painter.layout_no_wrap(texto, fonte.clone(), aviso);
                    let caixa = egui::Rect::from_min_size(s + egui::vec2(10., -gal.size().y - 10.), gal.size()).expand(4.);
                    painter.circle_stroke(s, 7., egui::Stroke::new(2., aviso));
                    painter.line_segment([s, caixa.left_bottom()], egui::Stroke::new(1., aviso));
                    painter.rect_filled(caixa, 4., fundo);
                    painter.galley(caixa.min + egui::vec2(4., 4.), gal, aviso);
                }
            }
        }
        linhas.push(format!("• {rotulo}"));
    }
    if falta.outros > 0 {
        linhas.push(format!("• {} {}", falta.outros, tr("GDL de peças sem junta ou de graus sem comando direto")));
    }
    let titulo = format!("{} ({com} GDL)", tr("Falta comandar"));
    let texto = std::iter::once(titulo).chain(linhas).collect::<Vec<_>>().join("\n");
    let gal = painter.layout_no_wrap(texto, fonte, aviso);
    let caixa = egui::Rect::from_min_size(rect.left_top() + egui::vec2(12., 12.), gal.size()).expand(6.);
    painter.rect_filled(caixa, 6., fundo);
    painter.rect_stroke(caixa, 6., egui::Stroke::new(1., aviso), egui::StrokeKind::Inside);
    painter.galley(caixa.min + egui::vec2(6., 6.), gal, aviso);
}

#[cfg(test)]
mod tests {
    use super::marcas;

    #[test]
    fn marcas_redondas_cobrem_a_faixa() {
        assert_eq!(marcas(0., 2., 5), vec![0., 0.5, 1., 1.5, 2.]);
        assert_eq!(marcas(0., 6., 5), vec![0., 2., 4., 6.]);
        assert_eq!(marcas(-3., 47., 4), vec![0., 20., 40.]);
        assert_eq!(marcas(0., 0.03, 3), vec![0., 0.01, 0.02, 0.03]);
        // faixa vazia (grandeza parada) ainda ganha eixo
        assert_eq!(marcas(5., 5., 4), vec![4., 4.5, 5., 5.5, 6.]);
    }
}
