// SPDX-License-Identifier: LGPL-2.1-or-later
//! Interface do PLM nas ondas 3 e 4: checador de qualidade do modelo (B305) e comparação de versões
//! (B214). A decisão mora em `fenkra_project` (`qualidade_modelo`, `qualidade_projeto`, `comparacao`);
//! aqui ficam as medidas tiradas do recálculo, o menu da linha do histórico e as janelas.
//! Cores só por tema; o estado das janelas fica na memória temporária do egui.

use super::*;
use fenkra_project::comparacao::{ComparacaoVersoes, Mudanca};
use fenkra_project::qualidade_modelo::{Acesso, Gravidade, Medidas, Momento, Perfil, Regra};

/// Face com menos área que isso (mm²) é ruído de modelagem, não detalhe de projeto.
const FACE_MINUSCULA_MM2: f64 = 0.01;
/// Aresta mais curta que isso (mm) quase sempre vem de uma costura ou de um corte quase coincidente.
const ARESTA_MINUSCULA_MM: f64 = 0.05;

/// Medidas de qualidade de uma peça a partir do recálculo e do documento que o gerou.
pub(crate) fn medidas_do_resultado(r: &RecomputeResult, doc: &Document) -> Medidas {
    use fenkra_core::FeatureKind;
    let mut m = Medidas { tempo_recalculo_ms: r.elapsed_ms, tamanho_arquivo_kb: doc.to_json().len() as f64 / 1024., ..Default::default() };
    for b in &r.bodies {
        m.faces += b.faces as u64;
        m.triangulos += (b.mesh.indices.len() / 3) as u64;
        m.faces_minusculas += b.mesh.face_info.iter().filter(|f| f.area < FACE_MINUSCULA_MM2).count() as u64;
        // aresta = polilinha de segmentos (6 números cada); o comprimento é a soma dos segmentos
        let total = b.mesh.edges.len() / 6;
        for (i, &ini) in b.mesh.edge_ranges.iter().enumerate() {
            let fim = b.mesh.edge_ranges.get(i + 1).map(|&x| x as usize).unwrap_or(total);
            let comp: f64 = (ini as usize..fim.min(total))
                .map(|s| {
                    let g = &b.mesh.edges[s * 6..s * 6 + 6];
                    (((g[3] - g[0]) as f64).powi(2) + ((g[4] - g[1]) as f64).powi(2) + ((g[5] - g[2]) as f64).powi(2)).sqrt()
                })
                .sum();
            if comp < ARESTA_MINUSCULA_MM {
                m.arestas_minusculas += 1;
            }
        }
        if !b.mass.volume.is_finite() || (b.faces > 0 && b.mesh.indices.is_empty()) {
            m.geometria_invalida += 1;
        }
    }
    use fenkra_solidos::erro::ErrorKind;
    for (_, k) in r.tipos_de_erro(doc) {
        match k {
            ErrorKind::ReferenciaPerdida { .. } | ErrorKind::GeometriaPerdida => m.referencias_em_erro += 1,
            ErrorKind::Propria => m.geometria_invalida += 1,
            ErrorKind::PaiFalhou { .. } => {}
        }
    }
    for f in doc.features.iter().filter(|f| !f.suppressed) {
        match &f.kind {
            FeatureKind::Hole(h) if h.thread.is_some() && h.thread_modeled => m.roscas_modeladas += 1,
            FeatureKind::Thread(t) if t.modeled => m.roscas_modeladas += 1,
            FeatureKind::MeshImport(_) => m.malhas_importadas += 1,
            _ => {}
        }
    }
    m
}

fn cor_gravidade(ui: &egui::Ui, g: Gravidade) -> egui::Color32 {
    match g {
        Gravidade::Bloqueia => ui.visuals().error_fg_color,
        Gravidade::Avisa => ui.visuals().warn_fg_color,
        Gravidade::Informa => ui.visuals().weak_text_color(),
    }
}

/// Dados da exceção que está sendo escrita para uma regra.
#[derive(Clone, Default)]
struct ExcecaoUi {
    regra: Option<Regra>,
    nome: String,
    motivo: String,
}

#[derive(Clone)]
struct ComparacaoUi {
    id: u64,
    /// números das versões comparadas (a partir de 1), na ordem em que a pessoa pediu
    a: usize,
    b: usize,
    resultado: ComparacaoVersoes,
}

fn id_comparacao() -> egui::Id {
    egui::Id::new("fenkra_comparacao_versoes")
}

impl App {
    /// Medidas da peça aberta, se ela é a do documento do projeto e já foi recalculada.
    pub(super) fn medidas_da_peca_aberta(&self, id: u64) -> Option<Medidas> {
        if self.open.entry != Some(id) {
            return None;
        }
        self.result.as_ref().map(|r| medidas_do_resultado(r, &self.open.doc))
    }

    /// Seção "Qualidade do modelo" do painel do projeto: relatório do perfil, troca de perfil e
    /// exceções com nome e motivo (B305).
    pub(super) fn secao_qualidade(&mut self, ui: &mut egui::Ui, sel: u64) {
        let Some(p) = self.project.as_ref() else { return };
        let perfil = p.perfil_qualidade();
        let acesso = p.acesso_qualidade();
        let medidas = self.medidas_da_peca_aberta(sel);
        let relatorio = medidas.as_ref().and_then(|m| p.avaliar_qualidade(sel, m, Momento::Revisar).ok());
        let cab = ui.horizontal(|ui| {
            ui.add(egui::Image::new(crate::icons::get("qualidade_modelo")).fit_to_exact_size(egui::vec2(18., 18.)));
            ui.label(RichText::new(tr("Qualidade do modelo")).strong());
            if let Some(r) = &relatorio {
                let (txt, g) = if r.bloqueia() { (tr("bloqueia"), Gravidade::Bloqueia) } else if r.avisos().next().is_some() { (tr("avisos"), Gravidade::Avisa) } else { (tr("ok"), Gravidade::Informa) };
                ui.label(RichText::new(txt).small().color(cor_gravidade(ui, g)));
            }
        });
        cab.response.on_hover_text(tr("Mede o peso e a saúde da peça e compara com o perfil do projeto. Roda ao liberar a revisão."));
        egui::CollapsingHeader::new(tr("Detalhes da qualidade")).id_salt(("qualidade", sel)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Perfil"));
                let mut novo: Option<Perfil> = None;
                egui::ComboBox::from_id_salt(("perfil_qualidade", sel)).selected_text(tr(&perfil.nome)).show_ui(ui, |ui| {
                    for modelo in [Perfil::informativo(), Perfil::padrao(), Perfil::rigoroso()] {
                        let atual = modelo.nome == perfil.nome;
                        if ui.selectable_label(atual, tr(&modelo.nome)).clicked() && !atual {
                            // trocar de modelo conserva o cadeado do perfil em vigor
                            novo = Some(Perfil { travado: perfil.travado, ..modelo });
                        }
                    }
                });
                if let Some(n) = novo {
                    if let Err(e) = self.project.as_mut().unwrap().definir_perfil_qualidade(n) {
                        self.toast(e.to_string(), true);
                    }
                }
                let mut travado = perfil.travado;
                let r = ui.add_enabled(acesso == Acesso::Administrador, egui::Checkbox::new(&mut travado, tr("Travado")));
                r.on_hover_text(tr("Só o administrador trava. Com o perfil travado, cada usuário só pode deixá-lo mais rígido."));
                if travado != perfil.travado {
                    let mut n = perfil.clone();
                    n.travado = travado;
                    if let Err(e) = self.project.as_mut().unwrap().definir_perfil_qualidade(n) {
                        self.toast(e.to_string(), true);
                    }
                }
            });
            let Some(r) = relatorio else {
                ui.label(RichText::new(tr("Abra o documento e deixe recalcular para medir.")).weak().small());
                return;
            };
            if r.violacoes.is_empty() {
                ui.label(RichText::new(tr("Nenhuma regra violada.")).color(ui.visuals().hyperlink_color));
            }
            let mut st: ExcecaoUi = ui.ctx().data(|d| d.get_temp(egui::Id::new(("excecao_qualidade", sel)))).unwrap_or_default();
            let mut conceder = false;
            for v in &r.violacoes {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{} {}", tr(v.efetiva().rotulo()), tr(v.regra.rotulo()))).color(cor_gravidade(ui, v.efetiva())));
                    ui.label(RichText::new(format!("{} / {}", fmt_num(v.medido), fmt_num(v.limite))).weak().small());
                    match &v.excecao {
                        Some(e) => {
                            ui.label(RichText::new(format!("{}: {} ({})", tr("exceção de"), e.nome, e.motivo)).small());
                        }
                        None if v.gravidade == Gravidade::Bloqueia => {
                            if ui.small_button(tr("Registrar exceção…")).clicked() {
                                st.regra = Some(v.regra);
                            }
                        }
                        None => {}
                    }
                });
            }
            if let Some(regra) = st.regra {
                ui.group(|ui| {
                    ui.label(RichText::new(format!("{}: {}", tr("Exceção para"), tr(regra.rotulo()))).strong());
                    ui.add(egui::TextEdit::singleline(&mut st.nome).hint_text(tr("Nome de quem aceita")).desired_width(240.));
                    ui.add(egui::TextEdit::multiline(&mut st.motivo).hint_text(tr("Motivo")).desired_rows(2).desired_width(240.));
                    ui.label(RichText::new(tr("Vale só para esta versão e fica no diário do projeto.")).weak().small());
                    ui.horizontal(|ui| {
                        let ok = !st.nome.trim().is_empty() && !st.motivo.trim().is_empty();
                        conceder = ui.add_enabled(ok, egui::Button::new(tr("Registrar"))).clicked();
                        if ui.button(tr("Cancelar")).clicked() {
                            st = ExcecaoUi::default();
                        }
                    });
                });
            }
            if conceder {
                if let Some(regra) = st.regra {
                    match self.project.as_mut().unwrap().conceder_excecao_qualidade(sel, regra, &st.nome, &st.motivo) {
                        Ok(()) => self.toast(tr("Exceção registrada no diário"), false),
                        Err(e) => self.toast(e.to_string(), true),
                    }
                    st = ExcecaoUi::default();
                }
            }
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("excecao_qualidade", sel)), st));
        });
    }

    /// Lista curta das violações de um relatório, para os diálogos que rodam o checador (B305).
    pub(super) fn linhas_qualidade(&self, ui: &mut egui::Ui, r: &fenkra_project::qualidade_modelo::Relatorio) {
        if r.dispensado {
            return;
        }
        ui.label(RichText::new(tr("Qualidade do modelo")).strong());
        if r.violacoes.is_empty() {
            ui.label(RichText::new(tr("Nenhuma regra violada.")).color(ui.visuals().hyperlink_color).small());
        }
        for v in &r.violacoes {
            ui.label(RichText::new(format!("{} {} ({} / {})", tr(v.efetiva().rotulo()), tr(v.regra.rotulo()), fmt_num(v.medido), fmt_num(v.limite))).small().color(cor_gravidade(ui, v.efetiva())));
        }
    }

    /// Menu da linha de uma versão no histórico (B214): abrir somente leitura, restaurar como nova
    /// versão, comparar com a atual e comparar com outra.
    pub(super) fn menu_versao(&mut self, ui: &mut egui::Ui, sel: u64, numero: usize, total: usize, restaurar: &mut Option<usize>) {
        let hash = self.project.as_ref().and_then(|p| p.entry(sel).ok()).and_then(|e| e.versions.get(numero - 1).map(|v| v.hash.clone()));
        let Some(hash) = hash else { return };
        let hash_atual = self.project.as_ref().and_then(|p| p.entry(sel).ok()).and_then(|e| e.versions.last().map(|v| v.hash.clone()));
        let r = ui.menu_button("⋯", |ui| {
            if ui.button(tr("Abrir somente leitura")).clicked() {
                self.open_entry(sel, Some((hash.clone(), format!("v{numero}"))));
                ui.close();
            }
            if ui.button(tr("Ver no navegador")).on_hover_text(tr("Abre esta versão no visualizador, sem abrir o Fenkra")).clicked() {
                self.visualizar_versao(sel, &hash, None);
                ui.close();
            }
            if numero != total && ui.button(tr("Restaurar como nova versão")).on_hover_text(tr("Cria uma versão nova com o conteúdo desta; o histórico não é reescrito")).clicked() {
                *restaurar = Some(numero);
                ui.close();
            }
            ui.separator();
            if numero != total && ui.button(tr("Comparar com a atual")).clicked() {
                self.abrir_comparacao(ui.ctx(), sel, numero, total);
                ui.close();
            }
            if let (true, Some(atual)) = (numero != total, hash_atual.as_ref()) {
                if ui.button(tr("Comparar no navegador com a atual")).on_hover_text(tr("Abre as duas versões no visualizador: o que foi adicionado, removido, movido e alterado")).clicked() {
                    self.visualizar_versao(sel, atual, Some(&hash));
                    ui.close();
                }
            }
            if total > 1 {
                ui.menu_button(tr("Comparar com outra"), |ui| {
                    for outra in (1..=total).rev().filter(|&o| o != numero) {
                        if ui.button(format!("v{outra}")).clicked() {
                            self.abrir_comparacao(ui.ctx(), sel, numero, outra);
                            ui.close();
                        }
                    }
                });
            }
        });
        r.response.on_hover_text(tr("Abrir, restaurar ou comparar esta versão"));
    }

    /// Sobrepõe no 3D duas versões (B214): abre a mais nova como o corpo da vista e põe por cima, em
    /// cores do tema, o que existe só numa delas. O cálculo roda em segundo plano.
    pub(super) fn sobrepor_no_3d(&mut self, id: u64, a: usize, b: usize) {
        let (antes, depois) = (a.min(b), a.max(b));
        let Some(p) = self.project.as_ref() else { return };
        let total = p.entry(id).map(|e| e.versions.len()).unwrap_or(0);
        let (Ok(ha), Ok(hb)) = (p.hash_da_versao(id, antes), p.hash_da_versao(id, depois)) else { return };
        let (da, db) = match (p.load_version(&ha), p.load_version(&hb)) {
            (Ok(x), Ok(y)) => (x, y),
            (Err(e), _) | (_, Err(e)) => return self.toast(e.to_string(), true),
        };
        let rotulo = format!("v{depois}");
        let aberta = self.open.entry == Some(id) && if depois == total { self.open.frozen.is_none() } else { self.open.frozen.as_deref() == Some(rotulo.as_str()) };
        if !aberta {
            self.open_entry(id, if depois == total { None } else { Some((hb, rotulo.clone())) });
        }
        crate::camada3d::calcular_sobreposicao(da, db, format!("v{antes}"), rotulo);
        self.toast(tr("Calculando a sobreposição…"), false);
    }

    fn abrir_comparacao(&mut self, ctx: &egui::Context, id: u64, a: usize, b: usize) {
        match self.project.as_ref().unwrap().comparar_versoes(id, a, b) {
            Ok(resultado) => ctx.data_mut(|d| {
                d.insert_temp(id_comparacao(), Some(ComparacaoUi { id, a, b, resultado }));
            }),
            Err(e) => self.toast(e.to_string(), true),
        }
    }

    /// Janela com o resultado da comparação: metadados das duas versões e o que mudou por feature.
    pub(super) fn janela_comparacao(&mut self, ctx: &egui::Context) {
        let Some(st) = ctx.data(|d| d.get_temp::<Option<ComparacaoUi>>(id_comparacao())).flatten() else { return };
        let c = &st.resultado;
        let mut aberta = true;
        let mut sobrepor = false;
        egui::Window::new(format!("{} {} ↔ {}", tr("Comparar versões"), c.antes.rotulo, c.depois.rotulo)).id(id_comparacao()).open(&mut aberta).default_width(460.).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(crate::icons::get("comparar_versoes")).fit_to_exact_size(egui::vec2(20., 20.)));
                for m in [&c.antes, &c.depois] {
                    let rev = m.revisao.as_ref().map(|r| format!(" · Rev {r}")).unwrap_or_default();
                    ui.label(RichText::new(format!("{}{}", m.rotulo, rev)).strong());
                }
            });
            if c.mesmo_conteudo {
                ui.label(tr("As duas versões têm o mesmo conteúdo."));
            }
            sobrepor = ui.button(tr("Sobrepor no 3D")).on_hover_text(tr("Mostra no 3D o que existe só numa das versões: vermelho no que saiu, verde no que entrou")).clicked();
            if !c.metadados.is_empty() {
                ui.label(RichText::new(tr("Metadados")).strong());
                for d in &c.metadados {
                    ui.label(RichText::new(format!("{}: {} → {}", tr(&d.campo), d.antes, d.depois)).small());
                }
            }
            ui.separator();
            ui.label(RichText::new(format!("{} ({})", tr("Modelo"), c.modelo.total())).strong());
            if c.modelo.e_igual() {
                ui.label(RichText::new(tr("Nenhuma diferença no modelo.")).weak());
            }
            egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                for f in &c.modelo.features {
                    let cor = match f.mudanca {
                        Mudanca::Adicionada | Mudanca::Reativada => ui.visuals().hyperlink_color,
                        Mudanca::Removida => ui.visuals().error_fg_color,
                        _ => ui.visuals().warn_fg_color,
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(tr(rotulo_mudanca(&f.mudanca))).color(cor).strong());
                        ui.label(format!("{} ({})", f.nome, f.tipo));
                    });
                    if let Mudanca::Renomeada { de, para } = &f.mudanca {
                        ui.label(RichText::new(format!("{de} → {para}")).weak().small());
                    }
                    for d in &f.detalhes {
                        ui.label(RichText::new(format!("    {}", d.frase())).small());
                    }
                    if f.omitidos > 0 {
                        ui.label(RichText::new(format!("    … +{}", f.omitidos)).weak().small());
                    }
                }
                for d in &c.modelo.parametros {
                    ui.label(RichText::new(format!("{} {}", tr("Parâmetro"), d.frase())).small());
                }
                for d in &c.modelo.propriedades {
                    ui.label(RichText::new(format!("{} {}", tr("Propriedade"), d.frase())).small());
                }
            });
        });
        if !aberta {
            ctx.data_mut(|d| d.insert_temp(id_comparacao(), None::<ComparacaoUi>));
        }
        if sobrepor {
            self.sobrepor_no_3d(st.id, st.a, st.b);
        }
    }
}

fn rotulo_mudanca(m: &Mudanca) -> &'static str {
    match m {
        Mudanca::Adicionada => "Adicionada",
        Mudanca::Removida => "Removida",
        Mudanca::Movida { .. } => "Movida",
        Mudanca::Suprimida => "Suprimida",
        Mudanca::Reativada => "Reativada",
        Mudanca::Renomeada { .. } => "Renomeada",
        Mudanca::Alterada => "Alterada",
    }
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0. { format!("{v:.0}") } else { format!("{v:.1}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::{BoolOp, FeatureKind, OriginPlane, PlaneRef, PrimKind, Primitive};

    fn caixa(doc: &mut Document, lado: &str) {
        let s = |x: &str| x.to_string();
        doc.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s(lado), b: s("20"), c: s("5"), d: s("0"), op: BoolOp::Add }));
    }

    #[test]
    fn medidas_de_uma_caixa_saem_do_recalculo() {
        let mut d = Document::new("Placa");
        caixa(&mut d, "40");
        let r = fenkra_solidos::recompute(&d);
        let m = medidas_do_resultado(&r, &d);
        assert_eq!(m.faces, 6);
        assert_eq!(m.triangulos, 12, "cada face plana são 2 triângulos");
        assert!(m.tamanho_arquivo_kb > 0.);
        assert_eq!((m.faces_minusculas, m.arestas_minusculas, m.geometria_invalida, m.referencias_em_erro), (0, 0, 0, 0));
        // o perfil padrão deixa a caixa passar
        let rel = fenkra_project::qualidade_modelo::avaliar(&Perfil::padrao(), &m, Momento::Revisar, &[]);
        assert!(rel.violacoes.is_empty(), "{:?}", rel.violacoes);
        // um perfil com teto de 4 faces barra a mesma peça
        let mut estreito = Perfil::padrao();
        estreito.regras.iter_mut().find(|l| l.regra == Regra::Faces).unwrap().limite = 4.;
        estreito.regras.iter_mut().find(|l| l.regra == Regra::Faces).unwrap().gravidade = Gravidade::Bloqueia;
        assert!(fenkra_project::qualidade_modelo::avaliar(&estreito, &m, Momento::Revisar, &[]).bloqueia());
    }

    #[test]
    fn caixa_minuscula_conta_faces_e_arestas_minusculas() {
        let mut d = Document::new("Pino");
        let s = |x: &str| x.to_string();
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: s("0.02"), b: s("0.02"), c: s("0.02"), d: s("0"), op: BoolOp::Add }));
        let m = medidas_do_resultado(&fenkra_solidos::recompute(&d), &d);
        assert_eq!(m.faces_minusculas, 6, "faces de 0,0004 mm²");
        assert_eq!(m.arestas_minusculas, 12, "arestas de 0,02 mm");
    }

    #[test]
    fn contagem_de_malha_e_rosca_ignora_o_suprimido() {
        let mut d = Document::new("P");
        let id = d.add_feature(FeatureKind::MeshImport(fenkra_core::MeshImport { path: "x.stl".into(), ..Default::default() }));
        let r = fenkra_solidos::recompute(&d);
        assert_eq!(medidas_do_resultado(&r, &d).malhas_importadas, 1);
        d.features.iter_mut().find(|f| f.id == id).unwrap().suppressed = true;
        assert_eq!(medidas_do_resultado(&fenkra_solidos::recompute(&d), &d).malhas_importadas, 0);
    }
}
