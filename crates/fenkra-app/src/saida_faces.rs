// SPDX-License-Identifier: LGPL-2.1-or-later
//! B197: a análise de saída e undercut por face, no mapa de inclinação.
//!
//! O mapa de inclinação (B180) pinta ponto a ponto. Aqui a pessoa escolhe pintar por face: cada face
//! na cor da faixa do pior ângulo dela, ou na cor da classe (cavidade, núcleo, lateral, ambígua),
//! com as faces em undercut sempre na cor de erro. A conta é `fenkra_solidos::dfm_saida` sobre a
//! malha de exibição do corpo (a ordem das faces é a dela), refeita só quando o corpo, a direção ou
//! o mínimo mudam. O painel mostra a contagem de cada faixa ou classe e se a peça sai do molde.

use crate::i18n::tr;
use crate::render::{MeshVertex, Scene};
use crate::sup_analise::{paradas, rampa};
use eframe::egui::{self, Color32, RichText};
use fenkra_solidos::dfm_geo::Classe;
use fenkra_solidos::dfm_saida::{self as saida, Analise, Faixa, Limites};
use fenkra_solidos::RecomputeResult;
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Modo {
    /// ponto a ponto, no shader (B180)
    #[default]
    Pontos,
    /// cada face pela faixa do pior ângulo dela
    Faixa,
    /// cada face pela classe
    Classe,
}

#[derive(Default)]
struct Estado {
    modo: Modo,
    /// (malha, direção, mínimo) da última conta
    chave: Option<(usize, [i32; 3], i32)>,
    analise: Option<Analise>,
    demo_lido: bool,
}

thread_local! {
    static ST: RefCell<Estado> = RefCell::new(Estado::default());
}

fn rgb(c: Color32) -> [f32; 3] {
    [c.r() as f32 / 255., c.g() as f32 / 255., c.b() as f32 / 255.]
}

/// Cor da faixa: as mesmas paradas do mapa ponto a ponto (destaque, ok, aviso, erro).
pub fn cor_faixa(p: &[[f32; 3]; 4], f: Faixa) -> [f32; 3] {
    rampa(
        p,
        match f {
            Faixa::Tampa => 0.,
            Faixa::Suficiente => 1. / 3.,
            Faixa::Justa => 2. / 3.,
            Faixa::SemSaida => 1.,
        },
    )
}

/// Cor da classe, só com cores do tema.
pub fn cor_classe(v: &egui::Visuals, c: Classe) -> Color32 {
    match c {
        Classe::Cavidade => crate::theme::accent(v),
        Classe::Nucleo => crate::theme::st_ok(),
        Classe::Lateral => crate::theme::st_warn(),
        Classe::Ambigua => v.weak_text_color(),
    }
}

fn nome_classe(c: Classe) -> &'static str {
    match c {
        Classe::Cavidade => "Cavidade",
        Classe::Nucleo => "Núcleo",
        Classe::Lateral => "Lateral",
        Classe::Ambigua => "Ambígua",
    }
}

const CLASSES: [Classe; 4] = [Classe::Cavidade, Classe::Nucleo, Classe::Lateral, Classe::Ambigua];

/// Seção do painel do mapa de inclinação: escolha do modo e o resumo da análise.
pub fn secao(ui: &mut egui::Ui) {
    ST.with(|st| {
        let mut e = st.borrow_mut();
        if !e.demo_lido {
            e.demo_lido = true;
            // captura sem mouse: FENKRA_DEMO_SAIDA=faixa|classe
            match std::env::var("FENKRA_DEMO_SAIDA").as_deref() {
                Ok("faixa") => e.modo = Modo::Faixa,
                Ok("classe") => e.modo = Modo::Classe,
                _ => {}
            }
        }
        ui.separator();
        ui.label(RichText::new(tr("Pintar")).small());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut e.modo, Modo::Pontos, tr("Ponto a ponto")).on_hover_text(tr("Cor de cada ponto pelo ângulo de saída ali."));
            ui.selectable_value(&mut e.modo, Modo::Faixa, tr("Por face")).on_hover_text(tr("Cada face na cor do pior ângulo dela: uma face curva que chega a ficar paralela à abertura fica em erro."));
            ui.selectable_value(&mut e.modo, Modo::Classe, tr("Classe")).on_hover_text(tr("Cavidade olha para a abertura, núcleo para o lado oposto, lateral fica paralela e ambígua tem parte de cada. Undercut em vermelho."));
        });
        if e.modo == Modo::Pontos {
            return;
        }
        let Some(a) = &e.analise else {
            ui.label(RichText::new(tr("A análise por face pede um corpo só.")).small().weak());
            return;
        };
        let vis = ui.visuals().clone();
        let p = paradas(&vis);
        let linha = |ui: &mut egui::Ui, cor: Color32, nome: &str, n: usize| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(14., 10.), egui::Sense::hover());
                ui.painter().rect_filled(r, 2., cor);
                ui.label(RichText::new(format!("{n}")).small().strong());
                ui.label(RichText::new(tr(nome)).small().weak());
            });
        };
        if e.modo == Modo::Faixa {
            for f in Faixa::TODAS {
                linha(ui, crate::theme::cor_da_escala(cor_faixa(&p, f)), f.nome(), a.contar_faixa(f));
            }
        } else {
            for c in CLASSES {
                linha(ui, cor_classe(&vis, c), nome_classe(c), a.contar_classe(c));
            }
        }
        linha(ui, crate::theme::st_err(), "Em undercut", a.undercuts().count());
        let (txt, cor) = if a.moldavel() { (tr("Sai do molde nesta direção."), crate::theme::st_ok()) } else { (tr("Não sai do molde assim: pede saída ou gaveta."), crate::theme::st_warn()) };
        ui.label(RichText::new(txt).small().color(cor));
    });
}

/// Faces pintadas por faixa ou classe, por cima do mapa.
pub fn decorar(settings: &crate::settings::Settings, r: &RecomputeResult, v: &egui::Visuals, s: &mut Scene) {
    if settings.display != crate::render::DisplayMode::Inclinacao {
        return;
    }
    let Some(b) = r.bodies.first().filter(|_| r.bodies.len() == 1) else {
        ST.with(|st| st.borrow_mut().analise = None);
        return;
    };
    let dir = crate::sup_inclinacao::direcao(settings.incl_eixo, settings.incl_inverte);
    let minimo = settings.incl_min.max(0.1) as f64;
    let m = &b.mesh;
    let p = paradas(v);
    ST.with(|st| {
        let mut e = st.borrow_mut();
        if e.modo == Modo::Pontos {
            return;
        }
        let chave = (m.positions.as_ptr() as usize ^ m.indices.len(), dir.map(|x| x as i32), (minimo * 100.) as i32);
        if e.chave != Some(chave) {
            e.analise = saida::analisar_malha(m, dir.map(|x| x as f64), Limites { saida_min: minimo, undercut_max: 0. });
            e.chave = Some(chave);
        }
        let Some(a) = &e.analise else { return };
        for f in &a.faces {
            let c = if a.em_undercut(f) {
                rgb(crate::theme::st_err())
            } else if e.modo == Modo::Faixa {
                cor_faixa(&p, a.faixa(f))
            } else {
                rgb(cor_classe(v, f.classe))
            };
            let ini = m.face_ranges.get(f.face).copied().unwrap_or(0) as usize;
            let fim = m.face_ranges.get(f.face + 1).map_or(m.indices.len(), |x| *x as usize);
            for &vi in m.indices.get(ini..fim).unwrap_or(&[]) {
                let k = vi as usize * 3;
                s.ghost.push(MeshVertex { pos: [m.positions[k], m.positions[k + 1], m.positions[k + 2]], nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [c[0], c[1], c[2], 0.97], mat: [1., 0., 1., 0.] });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixas_usam_as_paradas_do_mapa() {
        let p = [[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [1., 0., 0.]];
        assert_eq!(cor_faixa(&p, Faixa::Tampa), p[0]);
        assert_eq!(cor_faixa(&p, Faixa::SemSaida), p[3]);
        // as quatro faixas são distintas
        let v: Vec<_> = Faixa::TODAS.iter().map(|f| cor_faixa(&p, *f)).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(v[i], v[j]);
            }
        }
    }

    #[test]
    fn classes_tem_cores_distintas_no_tema_escuro() {
        let v = egui::Visuals::dark();
        let c: Vec<_> = CLASSES.iter().map(|x| cor_classe(&v, *x)).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(c[i], c[j], "{i} {j}");
            }
        }
    }
}
