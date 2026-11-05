// SPDX-License-Identifier: LGPL-2.1-or-later
//! Estrutural: painéis da placa de extremidade e do reforço (B255). A placa escolhe a face da ponta
//! (clique na face no 3D), o sentido externo ou interno com recuo, a sobra em mm ou em razão da parede
//! e furos de fixação. O reforço escolhe a aresta de encontro (clique na aresta), tem as abas D1 e D2
//! com inversão, perfil triangular, chanfrado ou poligonal, lado da espessura, posição com
//! afastamento e repetição por quantidade ou espaçamento.

use crate::editor::{check, expr, section};
use crate::geo_ui::Seed;
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::*;
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

fn vazio_vira(s: &mut String, padrao: &str) {
    if s.trim().is_empty() {
        *s = padrao.into();
    }
}

fn numero(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok()
}

/// Meio da aresta escolhida na viewport, se houver (primeira curva do tipo aresta da seleção).
pub fn meio_da_aresta(seed: &Seed) -> Option<[f64; 3]> {
    let pts = seed.curves.iter().find_map(|c| c.edge_pts())?;
    let (a, b) = (pts.first()?, pts.last()?);
    // a polilinha pode ser longa e curva: o ponto do meio da lista fica mais perto da aresta que a corda
    let m = pts.get(pts.len() / 2).copied().unwrap_or(*a);
    Some(if pts.len() > 2 { m } else { [0, 1, 2].map(|i| (a[i] + b[i]) / 2.) })
}

pub fn placa_panel(ui: &mut egui::Ui, p: &mut StructPlate, vars: &Vars, seed: &Seed) {
    section(ui, "Placa de extremidade");
    ui.label(RichText::new(format!("{} {}", tr("face da ponta perto de"), crate::app::p3(p.at, 1))).small());
    ui.label(RichText::new(tr("Clique a face da ponta do membro no 3D: a placa vai para a ponta que tem essa face.")).weak().small());
    if let Some(q) = seed.points.iter().find_map(|x| if let PointRef::Fixed(q) = x { Some(*q) } else { None }) {
        if ui.button(tr("⊙ Usar o vértice selecionado")).clicked() {
            p.at = q;
        }
    }
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Sentido")));
        let antes = p.side;
        ui.selectable_value(&mut p.side, PlateSide::Externo, tr("Externo")).on_hover_text(tr("A placa fecha a ponta por fora, além da face."));
        ui.selectable_value(&mut p.side, PlateSide::Interno, tr("Interno")).on_hover_text(tr("A placa entra no perfil, recuada da face da ponta."));
        if antes != p.side {
            // interna: nasce do tamanho do vão (sobra de menos uma parede); externa: volta a uma sobra em mm
            if p.side == PlateSide::Interno && numero(&p.margin).is_some_and(|v| v >= 0.) && p.margin_mode == MarginMode::Valor {
                p.margin = "-1".into();
                p.margin_mode = MarginMode::RazaoParede;
            } else if p.side == PlateSide::Externo && p.margin_mode == MarginMode::RazaoParede && numero(&p.margin).is_some_and(|v| v < 0.) {
                p.margin = "10".into();
                p.margin_mode = MarginMode::Valor;
            }
        }
    });
    if p.side == PlateSide::Interno {
        vazio_vira(&mut p.recess, "0");
        expr(ui, "Recuo da face", &mut p.recess, vars, "mm");
    }
    expr(ui, "Espessura", &mut p.thickness, vars, "mm");
    expr(ui, "Sobra", &mut p.margin, vars, if p.margin_mode == MarginMode::Valor { "mm" } else { "× parede" });
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Sobra em")));
        ui.selectable_value(&mut p.margin_mode, MarginMode::Valor, tr("mm"));
        ui.selectable_value(&mut p.margin_mode, MarginMode::RazaoParede, tr("razão da parede")).on_hover_text(tr("1 = uma espessura de parede do perfil; negativa encolhe a placa para dentro do vão."));
    });
    let mut furos = p.holes.is_some();
    if check(ui, &mut furos, "Furos de fixação").changed() {
        p.holes = furos.then(|| PlateHoles { diameter: "9".into(), edge: "15".into() });
    }
    if let Some(h) = &mut p.holes {
        expr(ui, "Diâmetro do furo", &mut h.diameter, vars, "mm");
        expr(ui, "Furo até a borda", &mut h.edge, vars, "mm");
        ui.label(RichText::new(tr("Quatro furos, um em cada canto da placa.")).weak().small());
    }
}

pub fn reforco_panel(ui: &mut egui::Ui, g: &mut StructGusset, vars: &Vars, seed: &Seed) {
    section(ui, "Reforço");
    ui.label(RichText::new(format!("{} {}", tr("encontro perto de"), crate::app::p3(g.at, 1))).small());
    ui.label(RichText::new(tr("Clique a aresta de encontro dos dois membros no 3D: o reforço vai para ela.")).weak().small());
    // clicar uma aresta nova no 3D leva o reforço para ela
    let chave = egui::Id::unique("st_reforco_aresta");
    if let Some(m) = meio_da_aresta(seed) {
        let ultimo: Option<[f64; 3]> = ui.ctx().data(|d| d.get_temp(chave));
        if ultimo != Some(m) {
            g.at = m;
            ui.ctx().data_mut(|d| d.insert_temp(chave, m));
        }
        if ui.button(tr("⊙ Usar a aresta selecionada")).clicked() {
            g.at = m;
        }
    } else if let Some(q) = seed.points.iter().find_map(|x| if let PointRef::Fixed(q) = x { Some(*q) } else { None }) {
        if ui.button(tr("⊙ Usar o vértice selecionado")).clicked() {
            g.at = q;
        }
    }
    section(ui, "Abas");
    expr(ui, "Aba D1", &mut g.leg, vars, "mm");
    let mut d2 = !g.leg2.trim().is_empty();
    if check(ui, &mut d2, "Aba D2 diferente da D1").changed() {
        g.leg2 = if d2 { g.leg.clone() } else { String::new() };
    }
    if d2 {
        expr(ui, "Aba D2", &mut g.leg2, vars, "mm");
    }
    check(ui, &mut g.swap, "Inverter D1 e D2");
    ui.label(RichText::new(tr("D1 corre pelo primeiro membro do encontro e D2 pelo segundo; a inversão troca.")).weak().small());
    expr(ui, "Espessura", &mut g.thickness, vars, "mm");
    section(ui, "Perfil do reforço");
    let forma = match (g.profile, numero(&g.chamfer)) {
        (GussetProfile::Poligonal, _) => 2,
        (_, Some(c)) if c.abs() < 1e-9 => 0,
        _ => 1,
    };
    ui.horizontal(|ui| {
        let mut nova = forma;
        ui.selectable_value(&mut nova, 0, tr("Triangular"));
        ui.selectable_value(&mut nova, 1, tr("Chanfrado"));
        ui.selectable_value(&mut nova, 2, tr("Poligonal"));
        if nova != forma {
            match nova {
                0 => {
                    g.profile = GussetProfile::Chanfrado;
                    g.chamfer = "0".into();
                }
                1 => {
                    g.profile = GussetProfile::Chanfrado;
                    if numero(&g.chamfer).is_some_and(|c| c.abs() < 1e-9) {
                        g.chamfer = "10".into();
                    }
                }
                _ => {
                    g.profile = GussetProfile::Poligonal;
                    if g.poly.is_empty() {
                        let d2 = if g.leg2.trim().is_empty() { g.leg.clone() } else { g.leg2.clone() };
                        g.poly = vec![["0".into(), "0".into()], [g.leg.clone(), "0".into()], ["0".into(), d2]];
                    }
                }
            }
        }
    });
    match forma {
        1 => {
            expr(ui, "Chanfro de alívio", &mut g.chamfer, vars, "mm");
            ui.label(RichText::new(tr("Chanfro maior que 0 corta o canto do encontro (passagem do cordão de solda).")).weak().small());
        }
        2 => poligono_ui(ui, g, vars),
        _ => {}
    }
    section(ui, "Posição ao longo da aresta");
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Lado da espessura")));
        ui.selectable_value(&mut g.side, GussetSide::Interno, tr("Interno"));
        ui.selectable_value(&mut g.side, GussetSide::DoisLados, tr("Dois lados"));
        ui.selectable_value(&mut g.side, GussetSide::Externo, tr("Externo"));
    });
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Posição")));
        ui.selectable_value(&mut g.position, GussetPos::Inicio, tr("Início"));
        ui.selectable_value(&mut g.position, GussetPos::Meio, tr("Meio"));
        ui.selectable_value(&mut g.position, GussetPos::Fim, tr("Fim"));
    });
    vazio_vira(&mut g.offset, "0");
    expr(ui, "Afastamento", &mut g.offset, vars, "mm");
    ui.label(RichText::new(tr("Início e fim encostam a face do reforço no afastamento; no meio, o lado da espessura decide de que lado do plano ele cresce.")).weak().small());
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Repetir")));
        ui.selectable_value(&mut g.repeat, GussetRepeat::Nenhuma, tr("Não"));
        ui.selectable_value(&mut g.repeat, GussetRepeat::Quantidade, tr("Por quantidade"));
        ui.selectable_value(&mut g.repeat, GussetRepeat::Espacamento, tr("Por espaçamento"));
    });
    match g.repeat {
        GussetRepeat::Quantidade => {
            vazio_vira(&mut g.count, "2");
            expr(ui, "Quantidade", &mut g.count, vars, "");
        }
        GussetRepeat::Espacamento => {
            vazio_vira(&mut g.spacing, "20");
            expr(ui, "Espaçamento", &mut g.spacing, vars, "mm");
        }
        GussetRepeat::Nenhuma => {}
    }
}

fn poligono_ui(ui: &mut egui::Ui, g: &mut StructGusset, vars: &Vars) {
    ui.label(RichText::new(tr("Vértices do contorno: u ao longo de D1 e v ao longo de D2, em mm a partir do canto do encontro.")).weak().small());
    let mut tirar = None;
    let n = g.poly.len();
    for (i, q) in g.poly.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}", i + 1));
            for (k, e) in q.iter_mut().enumerate() {
                let ok = fenkra_core::expr::eval_str(e, vars).is_ok();
                let mut t = egui::TextEdit::singleline(e).desired_width(60.);
                if !ok {
                    t = t.text_color(crate::theme::st_err());
                }
                ui.add(t).on_hover_text(if k == 0 { tr("u: ao longo de D1") } else { tr("v: ao longo de D2") });
            }
            if ui.add_enabled(n > 3, egui::Button::new("✖").small()).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        g.poly.remove(i);
    }
    if ui.button(tr("+ vértice")).clicked() {
        let ultimo = g.poly.last().cloned().unwrap_or(["0".into(), "0".into()]);
        g.poly.push(ultimo);
    }
}
