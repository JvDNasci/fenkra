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
    /// captura: comando fora de ordem, comando que falhou e plano com pai
    demo: Option<(Id, Id, Id)>,
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
        let mut troca: Option<dg::Troca> = None;
        let mut acao: Option<(u8, Id)> = None;
        let montagem = self.asm.as_ref().filter(|_| self.settings.bench == crate::settings::Bench::Montagem).map(|st| st.report.failed.iter().map(|id| (*id, st.asm.name_of(*id))).collect::<Vec<_>>());
        let mut na_montagem: Option<(u8, Id)> = None;
        egui::Window::new(tr("Diagnóstico")).open(&mut aberto).default_width(460.).default_pos([16., 130.]).show(ctx, |ui| {
            for (id, nome) in montagem.iter().flatten() {
                ui.group(|ui| {
                    ui.label(RichText::new(format!("✖ {nome}")).strong().color(crate::theme::st_err()));
                    ui.horizontal_wrapped(|ui| {
                        for (k, rot) in [(0u8, "Realçar"), (2, "Desativar"), (3, "Apagar")] {
                            if ui.small_button(tr(rot)).clicked() {
                                na_montagem = Some((k, *id));
                            }
                        }
                    });
                });
            }
            if falhas.is_empty() {
                if montagem.as_ref().is_none_or(|m| m.is_empty()) {
                    ui.label(tr("Nenhum comando com erro."));
                }
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
                            for (k, rot) in [(0u8, "Realçar"), (1, "Editar"), (2, "Desativar"), (3, "Apagar"), (4, "Pais e filhos"), (5, "Reordenar")] {
                                if ui.small_button(tr(rot)).clicked() {
                                    acao = Some((k, f.id));
                                }
                            }
                            if let Some(alvo) = dg::alvo_de_isolar(&self.open.doc, f.id) {
                                if ui.small_button(tr("Isolar")).on_hover_text(format!("{} {}", tr("Guarda a geometria de agora e corta os pais de"), self.open.doc.name_of(alvo))).clicked() {
                                    acao = Some((6, alvo));
                                }
                            }
                            // outro objeto da mesma família entra no lugar dele ou de um pai dele
                            let opcoes = dg::trocas(&self.open.doc, f.id);
                            if !opcoes.is_empty() {
                                ui.menu_button(tr("Substituir"), |ui| {
                                    for t in opcoes {
                                        if ui.button(&t.rotulo).clicked() {
                                            troca = Some(t);
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
        if let Some(t) = troca {
            dg::aplicar_troca(&mut self.open.doc, &t);
        }
        if let (Some((k, id)), Some(st)) = (na_montagem, self.asm.as_mut()) {
            match k {
                0 => st.sel = Some(id),
                2 => {
                    st.push_undo();
                    if let Some(m) = st.asm.mates.iter_mut().find(|m| m.id == id) {
                        m.suppressed = true;
                    }
                    if let Some(j) = st.asm.joints.iter_mut().find(|j| j.id == id) {
                        j.suppressed = true;
                    }
                    st.solve();
                }
                _ => {
                    st.push_undo();
                    st.asm.delete(id);
                    st.solve();
                }
            }
        }
    }

    /// `FENKRA_DEMO_JANELA=diagnostico`: peça com as três falhas que o painel resolve, e o painel
    /// aberto. `FENKRA_DEMO_DIAG=reordenar|substituir|isolar` faz o que o botão faria, para ver o depois.
    pub(super) fn demo_diagnostico(&mut self) {
        let mut d = Document::new("Diagnóstico");
        let mut n = 7_600_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut retangulo = SketchData::default();
        retangulo.add_rect(&mut al, [0., 0.], [60., 30.], false);
        let mut aberto = SketchData::default();
        let (p, q) = (aberto.add_point(&mut al, 0., 0.), aberto.add_point(&mut al, 40., 20.));
        aberto.add_curve(&mut al, fenkra_core::sketch::Curve::Line { a: p, b: q }, false);
        let extrude = |sketch: Id, z: &str| FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(z.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() });
        let base = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        let plano = d.add_offset_plane(PlaneRef::Datum(base), "10");
        let bom = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(plano), data: retangulo }));
        let corpo = d.add_feature(extrude(bom, "8"));
        let ruim = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: aberto }));
        let falha = d.add_feature(extrude(ruim, "4"));
        // comando antes do sketch dele: é o caso do Reordenar
        let tarde = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(plano), data: d.feature(bom).map_or_else(SketchData::default, |f| if let FeatureKind::Sketch(s) = &f.kind { s.data.clone() } else { SketchData::default() }) }));
        let fora = d.add_feature(extrude(tarde, "2"));
        if let (Some(i), Some(j)) = (d.feature_index(fora), d.feature_index(tarde)) {
            d.features.swap(i, j);
        }
        let _ = (corpo, falha);
        self.open.doc = d;
        self.sent_doc = None;
        self.diag.demo = Some((fora, falha, plano));
        self.diag.painel = true;
    }

    /// Faz a ação pedida em `FENKRA_DEMO_DIAG`, depois de a peça estar calculada.
    pub(super) fn demo_diagnostico_acao(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.time) < 4.5 {
            return;
        }
        let Some((fora, falha, plano)) = self.diag.demo.take() else { return };
        match std::env::var("FENKRA_DEMO_DIAG").as_deref() {
            Ok("reordenar") => {
                if let Err(e) = depois_dos_pais(&mut self.open.doc, fora) {
                    self.toast(tr(e).to_string(), true);
                }
            }
            Ok("substituir") => {
                if let Some(t) = dg::trocas(&self.open.doc, falha).into_iter().find(|t| t.alvo == Some(falha)) {
                    dg::aplicar_troca(&mut self.open.doc, &t);
                }
            }
            Ok("isolar") => {
                self.abrir_pais_filhos(plano);
                self.isolar(plano);
            }
            Ok("pais") => self.abrir_pais_filhos(plano),
            _ => {}
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
        egui::Window::new(titulo).id(egui::Id::new("fenkra_pais_filhos")).open(&mut aberto).default_width(340.).default_pos([490., 130.]).show(ctx, |ui| {
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
        let lista: Vec<Id> = dg::candidatos(&d, a, None).into_iter().map(|x| x.0).collect();
        assert_eq!(lista, vec![b]);
        dg::aplicar_troca(&mut d, &dg::Troca { alvo: None, de: a, para: b, rotulo: String::new() });
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

