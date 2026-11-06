// SPDX-License-Identifier: LGPL-2.1-or-later
//! Janelas de diagnóstico e de pais e filhos (B69) e o filtro da árvore.
//!
//! A lógica (quem falhou primeiro, quem é pai de quem, o que o filtro aceita) está em
//! `crate::diagnostico`; aqui só desenhamos e ligamos os botões às ações que a árvore já tem.

use super::*;
use crate::diagnostico as dg;

#[derive(Default)]
pub struct Estado {
    /// painel Diagnóstico aberto
    pub painel: bool,
    /// objeto cuja janela Pais e filhos está aberta
    pub pais_filhos: Option<Id>,
}

/// Elementos da mesma classe que podem entrar no lugar de `id`: quem depende dele fica de fora.
fn iguais_a(doc: &Document, id: Id) -> Vec<(Id, String)> {
    let Some(r) = doc.reference(id) else { return vec![] };
    let (classe, filhos) = (r.kind.class(), dg::filhos(doc, id));
    doc.references.iter().filter(|x| x.id != id && x.kind.class() == classe && !filhos.contains(&x.id)).map(|x| (x.id, x.name.clone())).collect()
}

/// Quem usava `de` passa a usar `para`.
fn substituir(doc: &mut Document, de: Id, para: Id) {
    let mut troca = |x: &mut Id| {
        if *x == de {
            *x = para;
        }
    };
    for f in &mut doc.features {
        f.kind.walk(&mut troca, &mut |_| {});
    }
    for r in doc.references.iter_mut().filter(|r| r.id != para) {
        r.kind.walk(&mut troca, &mut |_| {});
    }
}

/// Reordenar: o comando vai para logo depois do último pai. Falha sem mexer se isso quebrar a ordem.
fn depois_dos_pais(doc: &mut Document, id: Id) -> Result<(), &'static str> {
    let aqui = doc.feature_index(id).ok_or("O comando já está depois dos pais: a causa é outra.")?;
    let ultimo = dg::pais(doc, id).into_iter().filter_map(|p| doc.feature_index(p)).max();
    match ultimo {
        Some(u) if u > aqui => doc.move_feature(id, u).map_err(|_| "Um pai depende deste comando: mover quebraria a ordem."),
        _ => Err("O comando já está depois dos pais: a causa é outra."),
    }
}

/// Caixa do filtro da árvore (`:erro`, `:suprimido`, `:oculto` ou parte do nome); devolve a consulta.
pub fn caixa_de_filtro(ui: &mut egui::Ui) -> String {
    let chave = egui::Id::new("fenkra_filtro_arvore");
    let mut q: String = ui.ctx().data(|m| m.get_temp(chave)).unwrap_or_default();
    ui.horizontal(|ui| {
        let larg = ui.available_width() - 28.;
        crate::tree_ui::campo_discreto(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut q).desired_width(larg).hint_text(tr("Filtrar: nome, :erro, :suprimido, :oculto")));
        });
        if !q.is_empty() && ui.small_button("✕").on_hover_text(tr("Limpar o filtro")).clicked() {
            q.clear();
        }
    });
    ui.ctx().data_mut(|m| m.insert_temp(chave, q.clone()));
    q
}

/// Consulta atual do filtro (para a linha decidir se aparece).
pub fn filtro_atual(ctx: &egui::Context) -> dg::Filtro {
    let q: String = ctx.data(|m| m.get_temp(egui::Id::new("fenkra_filtro_arvore"))).unwrap_or_default();
    dg::Filtro::analisar(&q)
}

impl App {
    pub fn abrir_diagnostico(&mut self) {
        self.diag.painel = true;
    }

    pub fn abrir_pais_filhos(&mut self, id: Id) {
        self.diag.pais_filhos = Some(id);
    }

    fn realcar(&mut self, id: Id) {
        self.sel = if self.open.doc.feature(id).is_some() { Sel::Feature(id) } else { Sel::Ref(id) };
        self.scene_dirty = true;
    }

    fn alternar_supressao(&mut self, id: Id) {
        if let Some(f) = self.open.doc.feature_mut(id) {
            f.suppressed = !f.suppressed;
        }
    }

    /// Chamado a cada quadro, junto dos demais diálogos.
    pub fn janelas_de_diagnostico(&mut self, ctx: &egui::Context) {
        self.janela_diagnostico(ctx);
        self.janela_pais_filhos(ctx);
    }

    fn janela_diagnostico(&mut self, ctx: &egui::Context) {
        if !self.diag.painel {
            return;
        }
        let falhas = dg::falhas(&self.open.doc, |i| self.status_of(i));
        let mut aberto = true;
        let mut troca: Option<(Id, Id)> = None;
        let mut acao: Option<(u8, Id)> = None;
        egui::Window::new(tr("Diagnóstico")).open(&mut aberto).default_width(460.).show(ctx, |ui| {
            if falhas.is_empty() {
                ui.label(tr("Nenhum comando com erro."));
                return;
            }
            ui.label(RichText::new(tr("Do primeiro ao último: corrija a causa e as consequências somem sozinhas.")).weak().small());
            ui.add_space(4.);
            egui::ScrollArea::vertical().max_height(360.).show(ui, |ui| {
                for f in &falhas {
                    let nome = self.open.doc.name_of(f.id);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("✖ {nome}")).strong().color(crate::theme::st_err()));
                            if let Some(c) = f.causa {
                                ui.label(RichText::new(format!("{} {}", tr("consequência de"), self.open.doc.name_of(c))).weak().small());
                            } else {
                                ui.label(RichText::new(tr("causa")).small());
                            }
                        });
                        ui.label(RichText::new(&f.mensagem).small());
                        ui.horizontal_wrapped(|ui| {
                            for (k, rot) in [(0u8, "Realçar"), (1, "Editar"), (2, "Desativar"), (3, "Apagar"), (4, "Pais e filhos"), (5, "Reordenar"), (6, "Isolar")] {
                                if ui.small_button(tr(rot)).clicked() {
                                    acao = Some((k, f.id));
                                }
                            }
                            // elemento de geometria: outro da mesma classe entra no lugar dele
                            let iguais = iguais_a(&self.open.doc, f.id);
                            if !iguais.is_empty() {
                                ui.menu_button(tr("Substituir"), |ui| {
                                    for (id, nome) in iguais {
                                        if ui.button(nome).clicked() {
                                            troca = Some((f.id, id));
                                            ui.close();
                                        }
                                    }
                                });
                            }
                            if let Some(c) = f.causa {
                                if ui.small_button(tr("Ir à causa")).clicked() {
                                    acao = Some((0, c));
                                }
                            }
                        });
                    });
                }
            });
        });
        if !aberto {
            self.diag.painel = false;
        }
        match acao {
            Some((0, id)) => self.realcar(id),
            Some((1, id)) => self.start_session(id, false, None),
            Some((2, id)) => self.alternar_supressao(id),
            Some((3, id)) => self.ask_delete(id),
            Some((4, id)) => self.abrir_pais_filhos(id),
            Some((5, id)) => {
                if let Err(e) = depois_dos_pais(&mut self.open.doc, id) {
                    self.toast(tr(e).to_string(), true);
                }
            }
            Some((6, id)) => self.isolar(id),
            _ => {}
        }
        if let Some((de, para)) = troca {
            substituir(&mut self.open.doc, de, para);
        }
    }

    /// Isolar: o elemento troca a construção pela geometria de agora e deixa de depender dos pais.
    fn isolar(&mut self, id: Id) {
        let geo = self.result.as_ref().and_then(|r| {
            let plano = r.planes.iter().find(|p| p.id == Some(id)).map(|p| LinkedGeo::Plane { origin: p.frame.origin, x: p.frame.x, normal: p.frame.normal });
            plano.or_else(|| r.points.iter().find(|p| p.id == id).map(|p| LinkedGeo::Point(p.p))).or_else(|| r.curves.iter().find(|c| c.id == id).map(|c| LinkedGeo::Curve(c.pts.clone())))
        });
        let geo = geo.or_else(|| fenkra_solidos::surface_brep(&self.open.doc, id).map(LinkedGeo::Surface));
        match (geo, self.open.doc.references.iter_mut().find(|r| r.id == id)) {
            (Some(geo), Some(r)) => r.kind = RefKind::Isolated { geo },
            _ => self.toast(tr("Só um elemento de geometria já calculado pode ser isolado."), true),
        }
    }

    fn janela_pais_filhos(&mut self, ctx: &egui::Context) {
        let Some(id) = self.diag.pais_filhos else { return };
        if self.open.doc.feature(id).is_none() && self.open.doc.reference(id).is_none() {
            self.diag.pais_filhos = None;
            return;
        }
        let (pais, filhos) = (dg::pais(&self.open.doc, id), dg::filhos(&self.open.doc, id));
        let mut aberto = true;
        let mut ir: Option<Id> = None;
        let titulo = format!("{} - {}", tr("Pais e filhos"), self.open.doc.name_of(id));
        let geometria = self.open.doc.reference(id).is_some();
        let mut isolar = false;
        egui::Window::new(titulo).id(egui::Id::new("fenkra_pais_filhos")).open(&mut aberto).default_width(340.).show(ctx, |ui| {
            if geometria && ui.button(tr("Isolar")).on_hover_text(tr("O elemento guarda a geometria de agora e deixa de depender dos pais.")).clicked() {
                isolar = true;
            }
            for (rot, lista) in [(tr("Pais (de que depende)"), &pais), (tr("Filhos (quem depende dele)"), &filhos)] {
                ui.label(RichText::new(rot).strong());
                if lista.is_empty() {
                    ui.label(RichText::new(tr("nenhum")).weak());
                }
                for &o in lista {
                    ui.horizontal(|ui| {
                        let erro = matches!(self.status_of(o), Some(Status::Error(_)));
                        let txt = format!("{} ({})", self.open.doc.name_of(o), tr(dg::tipo_de(&self.open.doc, o)));
                        ui.label(if erro { RichText::new(format!("✖ {txt}")).color(crate::theme::st_err()) } else { RichText::new(txt) });
                        if ui.small_button(tr("Realçar")).clicked() {
                            ir = Some(o);
                        }
                    });
                }
                ui.add_space(6.);
            }
            ui.label(RichText::new(tr("Apagar este objeto leva os filhos junto; edite ou substitua os pais antes de apagar.")).weak().small());
        });
        if !aberto {
            self.diag.pais_filhos = None;
        }
        if isolar {
            self.isolar(id);
        }
        if let Some(o) = ir {
            self.realcar(o);
        }
    }
}

#[cfg(test)]
mod testes_acoes {
    use super::*;

    fn ponto(d: &mut Document, x: &str) -> Id {
        d.add_geometry(RefKind::PointCoords { x: x.into(), y: "0".into(), z: "0".into(), frame: None })
    }

    #[test]
    fn substituir_troca_quem_usava_e_nao_oferece_filho() {
        let mut d = Document::new("t");
        let (a, b) = (ponto(&mut d, "1"), ponto(&mut d, "2"));
        let filho = d.add_geometry(RefKind::PointBetween { a: PointRef::Ref(a), b: PointRef::Origin, ratio: "0".into() });
        let lista: Vec<Id> = iguais_a(&d, a).into_iter().map(|x| x.0).collect();
        assert_eq!(lista, vec![b]);
        substituir(&mut d, a, b);
        assert!(matches!(d.reference(filho).unwrap().kind, RefKind::PointBetween { a: PointRef::Ref(x), .. } if x == b));
    }

    #[test]
    fn reordenar_leva_para_depois_do_pai_e_recusa_sem_motivo() {
        let mut d = Document::new("t");
        let s = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: SketchData::default() }));
        let e = d.add_feature(FeatureKind::Extrude(Extrude { sketch: s, op: BoolOp::Add, extent: Extent::Blind("1".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        assert!(depois_dos_pais(&mut d, e).is_err(), "já está depois do sketch");
        assert_eq!(d.feature_index(e), Some(1));
    }
}

