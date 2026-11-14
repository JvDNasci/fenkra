// SPDX-License-Identifier: LGPL-2.1-or-later
//! B148: painel dos recursos de plástico (boss, snap-fit, grelha, lip).
//!
//! O painel abre na coluna da esquerda como os outros comandos (o editor o chama por
//! `feat_ui::panel`). A ordem é a do padrão: suporte (o sketch), parâmetros e avançado. Os campos
//! que a regra plástica governa usam `plastico_ui::campo`: com regra na peça ficam presos à fórmula,
//! e só aceitam número próprio em modo avançado. A prévia é a do comando aberto (fantasma no 3D) e a
//! seta da altura sai de `handles`.

use crate::editor::{check, expr, section};
use crate::i18n::tr;
use eframe::egui::{self, RichText};
use fenkra_core::recurso_plastico::*;
use fenkra_core::regra_plastica::Grandeza;
use fenkra_core::{Document, FeatureKind, Id};
use std::collections::HashMap;

type Vars = HashMap<String, f64>;

/// Tipos de recurso, na ordem da família na barra.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Boss,
}

/// Comando novo: nasce com as fórmulas da regra quando a peça tem uma.
pub fn criar(doc: &Document, t: Tipo, sketch: Id) -> FeatureKind {
    let r = doc.plastico.as_ref();
    FeatureKind::Plastico(match t {
        Tipo::Boss => RecursoPlastico::Boss(Boss::novo(sketch, r)),
    })
}

pub fn icone(p: &RecursoPlastico) -> &'static str {
    match p {
        RecursoPlastico::Boss(_) => "pl_boss",
    }
}

fn nota(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(tr(t)).weak().small());
}

/// Campo de texto opcional: desligado fica vazio (o recálculo usa o automático).
fn opcional(ui: &mut egui::Ui, rotulo_check: &str, rotulo: &str, s: &mut String, padrao: &str, vars: &Vars, unidade: &str) {
    let mut on = !s.trim().is_empty();
    if check(ui, &mut on, rotulo_check).changed() {
        *s = if on { padrao.to_string() } else { String::new() };
    }
    if on {
        expr(ui, rotulo, s, vars, unidade);
    }
}

pub fn painel(ui: &mut egui::Ui, doc: &Document, idx: usize, p: &mut RecursoPlastico, vars: &Vars) {
    let regra = doc.plastico.as_ref();
    match p {
        RecursoPlastico::Boss(b) => {
            section(ui, "Suporte");
            crate::feat_ui::sketch_pick(ui, "pl_boss_sk", "Centros", doc, idx, &mut b.sketch);
            nota(ui, "Cada ponto do sketch vira um boss que cresce pela normal do plano.");
            check(ui, &mut b.reverse, "para o outro lado do plano");
            section(ui, "Parâmetros");
            expr(ui, "Diâmetro do boss", &mut b.diametro, vars, "mm");
            expr(ui, "Altura do boss", &mut b.altura, vars, "mm");
            crate::plastico_ui::campo(ui, regra, "Saída do boss", &mut b.saida, Grandeza::Saida, vars, "°");
            crate::plastico_ui::campo(ui, regra, "Filete da base", &mut b.raio, Grandeza::Raio, vars, "mm");
            section(ui, "Furo e inserto");
            expr(ui, "Diâmetro do furo", &mut b.furo, vars, "mm");
            if fenkra_core::expr::eval_str(&b.furo, vars).is_ok_and(|v| v > 0.) {
                expr(ui, "Profundidade do furo", &mut b.furo_prof, vars, "mm");
            }
            let mut ins = !b.inserto.trim().is_empty();
            if check(ui, &mut ins, "rebaixo para inserto").changed() {
                (b.inserto, b.inserto_prof) = if ins { ("4.2".into(), "4".into()) } else { (String::new(), String::new()) };
            }
            if ins {
                expr(ui, "Diâmetro do inserto", &mut b.inserto, vars, "mm");
                expr(ui, "Profundidade do inserto", &mut b.inserto_prof, vars, "mm");
            }
            section(ui, "Nervuras");
            let d = fenkra_core::expr::eval_str(&b.diametro, vars).unwrap_or(0.);
            let h = fenkra_core::expr::eval_str(&b.altura, vars).unwrap_or(0.);
            let mut auto = b.nervuras.trim().is_empty();
            ui.horizontal(|ui| {
                if ui.radio_value(&mut auto, true, tr("automáticas")).on_hover_text(tr("Quatro nervuras quando a altura passa de 3 vezes o diâmetro; nenhuma abaixo disso.")).changed() {
                    b.nervuras.clear();
                }
                if ui.radio_value(&mut auto, false, tr("número")).changed() {
                    b.nervuras = "4".into();
                }
            });
            if !auto {
                expr(ui, "Número de nervuras", &mut b.nervuras, vars, "");
            } else {
                let n = if h > BOSS_NERVURA_DIAMETROS * d + 1e-9 { 4 } else { 0 };
                nota(ui, &format!("{} {n}", tr("nervuras agora:")));
            }
            crate::plastico_ui::campo(ui, regra, "Espessura da nervura", &mut b.nervura_esp, Grandeza::Nervura, vars, "mm");
            section(ui, "Avançado");
            opcional(ui, "largura própria da nervura", "Largura da nervura", &mut b.nervura_larg, &crate::app::fmt(d / 2.), vars, "mm");
            opcional(ui, "altura própria da nervura", "Altura da nervura", &mut b.nervura_alt, &crate::app::fmt(0.7 * h), vars, "mm");
            nota(ui, "Sem medida própria, a nervura vai até metade do diâmetro além da base e sobe 70% da altura.");
        }
    }
}

/// Variações da família de plástico na barra de recursos.
pub fn itens() -> Vec<crate::toolbar::Item> {
    use crate::app::Cmd;
    use crate::toolbar::{Act, Item};
    vec![Item {
        icon: "pl_boss",
        name: "Boss",
        brief: "Boss para parafuso ou inserto",
        advanced: "Pino oco em cada ponto do sketch, com saída e filete na base vindos da regra plástica, furo, rebaixo de inserto e nervuras de apoio quando ele fica alto.",
        act: Act::Cmd(Cmd::Plastico(Tipo::Boss)),
    }]
}

/// Seta de arraste no 3D: base, direção, expressão e rótulo do campo que ela muda.
pub fn seta(r: &fenkra_solidos::RecomputeResult, p: &RecursoPlastico) -> Option<(glam::Vec3, glam::Vec3, String, &'static str)> {
    let s = r.sketches.iter().find(|s| s.feature == p.sketch())?;
    let n = glam::Vec3::from(s.frame.normal.map(|x| x as f32));
    match p {
        RecursoPlastico::Boss(b) => {
            let base = glam::Vec3::from(*s.points.first()?);
            Some((base, if b.reverse { -n } else { n }, b.altura.clone(), "Altura do boss"))
        }
    }
}

pub fn definir_seta(p: &mut RecursoPlastico, valor: String) {
    match p {
        RecursoPlastico::Boss(b) => b.altura = valor,
    }
}
