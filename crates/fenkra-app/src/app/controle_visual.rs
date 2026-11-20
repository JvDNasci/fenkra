// SPDX-License-Identifier: LGPL-2.1-or-later
//! Controle visual (B440): um menu só para projeção, estilo de exibição, esconder e mostrar, e o
//! ambiente de ocultos. Abre pelo rótulo "Perspectiva · Sombreado" da barra de baixo e pelo menu
//! Exibir, com o mesmo conteúdo nos dois lugares.
//!
//! Esconder não muda o modelo: só a bandeira `visible` do corpo (e `hidden` da pasta ou do sketch).
//! Por isso o volume e a caixa envolvente do recálculo continuam iguais; o que muda é o que a cena
//! pinta e o que o ambiente de ocultos mostra (o inverso do normal).

use super::*;

/// B526: alterna a origem (planos e eixos); o estado mora no documento (`origin_visible`) e volta
/// igual ao reabrir o arquivo. Devolve o novo estado (`true` = visível).
pub(crate) fn alternar_origem(d: &mut Document) -> bool {
    d.origin_visible = !d.origin_visible;
    d.origin_visible
}

/// Corpos ocultos agora.
pub(crate) fn corpos_ocultos(d: &Document) -> usize {
    d.collectors.iter().filter(|c| !c.visible).count()
}

/// Mostra todos os corpos; devolve quantos estavam ocultos.
pub(crate) fn mostrar_todos_os_corpos(d: &mut Document) -> usize {
    let mut n = 0;
    for c in d.collectors.iter_mut().filter(|c| !c.visible) {
        c.visible = true;
        n += 1;
    }
    n
}

/// Deixa só `corpo` à vista (isolar); devolve quantos corpos mudaram.
pub(crate) fn isolar_corpo(d: &mut Document, corpo: Id) -> usize {
    if !d.collectors.iter().any(|c| c.id == corpo) {
        return 0;
    }
    let mut n = 0;
    for c in d.collectors.iter_mut() {
        let v = c.id == corpo;
        if c.visible != v {
            c.visible = v;
            n += 1;
        }
    }
    n
}

/// Corpo da seleção: o próprio corpo ou o corpo do comando de sólido escolhido.
pub(crate) fn corpo_da_selecao(d: &Document, sel: Sel) -> Option<Id> {
    match sel {
        Sel::Collector(id) => Some(id),
        Sel::Feature(id) => d.feature(id).filter(|f| !matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.target),
        _ => None,
    }
}

impl App {
    /// Conteúdo do controle visual, em três seções. As ações que já existem passam por `acts`.
    pub(super) fn controle_visual(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let fraco = |ui: &mut egui::Ui, t: &str| {
            ui.label(RichText::new(tr(t)).small().weak());
        };
        let ortho = self.viewport.as_ref().is_some_and(|v| v.camera.ortho);
        fraco(ui, "Projeção");
        if ui.selectable_label(!ortho, tr("Perspectiva")).clicked() && ortho {
            acts.push(Act::Ortho);
        }
        if ui.selectable_label(ortho, tr("Ortogonal")).clicked() && !ortho {
            acts.push(Act::Ortho);
        }
        ui.separator();
        fraco(ui, "Estilo de exibição");
        for m in crate::render::DisplayMode::ALL.into_iter().filter(|m| !m.e_analise()) {
            ui.radio_value(&mut self.settings.display, m, m.label());
        }
        ui.menu_button(tr("Análise"), |ui| {
            for m in crate::render::DisplayMode::ALL.into_iter().filter(|m| m.e_analise()) {
                ui.radio_value(&mut self.settings.display, m, m.label());
            }
        });
        ui.separator();
        fraco(ui, "Esconder e mostrar");
        let tem_sel = !matches!(self.sel, Sel::None | Sel::Part | Sel::Params) || self.multi.borrow().len() > 1;
        if ui.add_enabled(tem_sel, egui::Button::new(tr("Esconder ou mostrar a seleção   H"))).clicked() {
            self.toggle_hide_selected();
            ui.close();
        }
        let corpo = corpo_da_selecao(&self.open.doc, self.sel);
        if ui.add_enabled(corpo.is_some() && self.open.doc.collectors.len() > 1, egui::Button::new(tr("Esconder os outros corpos"))).on_hover_text(tr("Deixa à vista só o corpo selecionado, para trabalhar nele sem os outros na frente.")).clicked() {
            if let Some(c) = corpo {
                isolar_corpo(&mut self.open.doc, c);
                self.scene_dirty = true;
            }
            ui.close();
        }
        let ocultos = corpos_ocultos(&self.open.doc);
        if ui.add_enabled(ocultos > 0, egui::Button::new(format!("{} ({ocultos})", tr("Mostrar todos os corpos")))).clicked() {
            mostrar_todos_os_corpos(&mut self.open.doc);
            self.scene_dirty = true;
            ui.close();
        }
        let rotulo_origem = if self.open.doc.origin_visible { "Esconder a origem   Alt+O" } else { "Mostrar a origem   Alt+O" };
        if ui.button(tr(rotulo_origem)).on_hover_text(tr("Liga ou desliga os três planos e os eixos da origem de uma vez. A escolha fica gravada no arquivo; esconder não apaga nada.")).clicked() {
            acts.push(Act::Origem);
            ui.close();
        }
        ui.separator();
        fraco(ui, "Espaço dos ocultos");
        if ui.selectable_label(self.hidden_env, tr("Ambiente de ocultos   Shift+H")).on_hover_text(tr("Mostra só o que está oculto, para achar e trazer de volta o que foi escondido. Clique de novo para voltar ao normal.")).clicked() {
            acts.push(Act::HiddenEnv);
        }
    }
}

impl App {
    /// Desenvolvimento (B440), para a captura: `FENKRA_DEMO_JANELA=controle_visual` desenha o controle
    /// visual numa janela fixa; `=menu_janela` faz o mesmo com o menu Janela; `=opcoes_mouse` abre as
    /// configurações na navegação, com o perfil de `FENKRA_DEMO_NAV` (nome gravado, ex. `MeioGira`).
    /// Devolve `true` quando o nome é um destes.
    pub(super) fn demo_b440(&mut self, ctx: &egui::Context, nome: &str) -> bool {
        match nome {
            "controle_visual" | "menu_janela" => {
                egui::Window::new(tr(if nome == "menu_janela" { "Janela" } else { "Controle visual" })).default_pos([420., 140.]).resizable(false).show(ctx, |ui| {
                    if nome == "menu_janela" {
                        self.menu_janela(ui);
                    } else {
                        let mut acts = vec![];
                        self.controle_visual(ui, &mut acts);
                        acts.into_iter().for_each(|a| self.act(a));
                    }
                });
                true
            }
            "opcoes_mouse" => {
                static FEITO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !FEITO.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    self.settings_open = true;
                    self.settings_page = 3;
                    if let Ok(n) = std::env::var("FENKRA_DEMO_NAV") {
                        if let Some(s) = crate::settings::NavStyle::ALL.into_iter().find(|s| s.nome() == n) {
                            self.settings.nav = s;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, OriginPlane, PlaneRef, PrimKind, Primitive};

    /// Duas caixas, cada uma no seu corpo: 40 × 20 × 5 e 10 × 20 × 5.
    fn duas_caixas() -> (Document, Id, Id) {
        let mut d = Document::new("duas");
        let s = |x: &str| x.to_string();
        let a = d.collectors[0].id;
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("40"), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        let b = d.add_collector("B");
        d.active_collector = b;
        let f = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("10"), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
        d.feature_mut(f).unwrap().target = b;
        (d, a, b)
    }

    fn volume_total(d: &Document) -> f64 {
        let r = fenkra_solidos::recompute(d);
        assert!(r.status.values().all(|s| matches!(s, fenkra_solidos::Status::Ok)), "{:?}", r.status);
        r.bodies.iter().map(|b| b.mass.volume).sum()
    }

    /// Isolar, mostrar tudo e contar ocultos só mexem na visibilidade: o modelo recalculado tem o
    /// mesmo volume antes e depois.
    #[test]
    fn esconder_e_mostrar_nao_mudam_o_modelo() {
        let (mut d, a, b) = duas_caixas();
        assert_eq!(d.collectors.len(), 2);
        let antes = fenkra_kernel::AVAILABLE.then(|| volume_total(&d));
        if let Some(v) = antes {
            assert!((v - (40. * 20. * 5. + 10. * 20. * 5.)).abs() < 1e-3, "{v}");
        }
        assert_eq!(corpos_ocultos(&d), 0);
        assert_eq!(isolar_corpo(&mut d, a), 1);
        assert!(d.collectors.iter().find(|c| c.id == a).unwrap().visible);
        assert!(!d.collectors.iter().find(|c| c.id == b).unwrap().visible);
        assert_eq!(corpos_ocultos(&d), 1);
        // isolar de novo o mesmo corpo não muda nada; um corpo que não existe também não
        assert_eq!(isolar_corpo(&mut d, a), 0);
        assert_eq!(isolar_corpo(&mut d, Id(9999)), 0);
        if let Some(v) = antes {
            assert!((volume_total(&d) - v).abs() < 1e-6);
        }
        assert_eq!(mostrar_todos_os_corpos(&mut d), 1);
        assert_eq!(corpos_ocultos(&d), 0);
        assert_eq!(mostrar_todos_os_corpos(&mut d), 0);
    }

    #[test]
    fn corpo_da_selecao_vem_do_corpo_ou_do_comando_de_solido() {
        let (d, a, b) = duas_caixas();
        assert_eq!(corpo_da_selecao(&d, Sel::Collector(b)), Some(b));
        assert_eq!(corpo_da_selecao(&d, Sel::Feature(d.features[0].id)), Some(a));
        assert_eq!(corpo_da_selecao(&d, Sel::Part), None);
    }

    #[test]
    fn textos_do_controle_visual_tem_traducao() {
        for t in [
            "Projeção",
            "Estilo de exibição",
            "Análise",
            "Esconder e mostrar",
            "Esconder ou mostrar a seleção   H",
            "Esconder os outros corpos",
            "Deixa à vista só o corpo selecionado, para trabalhar nele sem os outros na frente.",
            "Mostrar todos os corpos",
            "Espaço dos ocultos",
            "Ambiente de ocultos   Shift+H",
            "Mostra só o que está oculto, para achar e trazer de volta o que foi escondido. Clique de novo para voltar ao normal.",
            "Controle visual",
            "Esconder a origem   Alt+O",
            "Mostrar a origem   Alt+O",
            "Mostrar ou esconder a origem  (Alt+O)",
            "Mostrar ou esconder a origem",
            "Origem visível",
            "Origem escondida",
            "Origem",
            "Elementos de construção",
        ] {
            assert!(crate::i18n::tem_traducao(t), "{t}");
        }
    }

    /// B526: o botão alterna a origem e o estado volta igual ao reabrir o arquivo.
    #[test]
    fn origem_alterna_e_volta_igual_ao_reabrir() {
        let mut d = Document::new("t");
        assert!(d.origin_visible, "a origem nasce visível");
        assert!(!alternar_origem(&mut d), "primeiro clique esconde");
        let reaberto = Document::from_json(&d.to_json()).unwrap();
        assert!(!reaberto.origin_visible, "escondida continua escondida ao reabrir");
        assert!(alternar_origem(&mut d), "segundo clique mostra");
        let reaberto = Document::from_json(&d.to_json()).unwrap();
        assert!(reaberto.origin_visible);
    }
}
