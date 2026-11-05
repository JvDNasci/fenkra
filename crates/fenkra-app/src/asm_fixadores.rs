// SPDX-License-Identifier: LGPL-2.1-or-later
//! Assistente de prendedores (B98): uma tabela com cada ponto da montagem, para trocar família,
//! tamanho, comprimento e pilha de uma vez, com um desfazer por alteração.
//!
//! A tabela lê as pilhas de `fenkra_montagem::pilhas` e manda nelas: cada mudança passa por
//! `pilhas::editar`, que refaz as peças. Linhas com o mesmo parafuso têm a mesma cor, e "aplicar aos
//! equivalentes" leva a mudança para todas elas. Escolher tamanho ou comprimento à mão desliga o
//! automático daquela linha, senão o recálculo seguinte desfaria a escolha.
//!
//! Ponto manual: com "Adicionar ponto" ligado, clicar numa face cria um prendedor ali, fora dos
//! furos. Na tabela ele mostra as coordenadas, e mudá-las move o prendedor.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Rect, RichText, Sense};
use fenkra_core::Id;
use fenkra_montagem::fixadores::OpcoesFixadores;
use fenkra_montagem::pilhas::{self, Diagnostico, Nivel, Pilha, Recalculo};
use fenkra_montagem::V3;
use fenkra_solidos::biblioteca::{self, Familia, TipoFixador};
use glam::Vec3;

pub const PAINEL: &str = "fixadores_assistente";

#[derive(Default)]
pub struct Estado {
    /// linha aberta (o ponto escolhido na árvore ou na tabela)
    pub sel: Option<Id>,
    pub equivalentes: bool,
    /// o próximo clique numa face cria um ponto manual
    pub adicionando: bool,
    /// tamanho e comprimento do próximo ponto manual
    pub novo: (String, f64),
}

/// Abre o assistente, na linha pedida, e já diagnostica.
pub fn abrir(st: &mut AsmState, sel: Option<Id>) {
    let antes = st.fixadores.take();
    let mut e = antes.unwrap_or(Estado { novo: ("M6".into(), 20.), ..Default::default() });
    e.sel = sel.or(e.sel);
    st.fixadores = Some(e);
    atualizar(st);
}

/// Acerta as pilhas com os furos. Se algo muda, é um passo de desfazer só.
pub fn atualizar(st: &mut AsmState) -> Option<&Recalculo> {
    if st.asm.pilhas.is_empty() {
        st.pilhas_diag = Recalculo::default();
        return None;
    }
    let dir = st.dir()?;
    let antes = st.asm.clone();
    let r = pilhas::recalcular(&mut st.asm, &dir, &mut st.cache, biblioteca::publica());
    if st.asm != antes {
        st.undo.push(antes);
        st.redo.clear();
        st.modified = true;
        st.solve();
    }
    st.pilhas_diag = r;
    Some(&st.pilhas_diag)
}

/// Depois de mudar uma peça: o aviso que a pessoa precisa ver, se algum ponto não fecha.
pub fn apos_mudar_peca(st: &mut AsmState) -> Option<String> {
    let r = atualizar(st)?;
    let erros = r.diagnosticos.iter().filter(|d| d.nivel == Nivel::Erro).count();
    if erros > 0 {
        return Some(format!("{erros} {}", tr("prendedor(es) com problema: abra o assistente")));
    }
    (r.refeitas + r.removidas > 0).then(|| format!("{} {}", r.refeitas + r.removidas, tr("prendedor(es) acompanharam os furos")))
}

/// Aplica `f` às pilhas, com um desfazer, e guarda o diagnóstico.
fn mudar(st: &mut AsmState, ids: &[Id], f: impl Fn(&mut Pilha)) {
    let Some(dir) = st.dir() else { return };
    st.push_undo();
    st.pilhas_diag = pilhas::editar(&mut st.asm, &dir, &mut st.cache, biblioteca::publica(), ids, f);
    st.solve();
}

/// Linhas que a mudança alcança: a própria e, se pedido, as equivalentes.
fn alcance(a: &fenkra_montagem::Assembly, id: Id, equivalentes: bool) -> Vec<Id> {
    let Some(p) = a.pilhas.iter().find(|p| p.id == id) else { return vec![] };
    a.pilhas.iter().filter(|q| q.id == id || (equivalentes && q.equivalente(p))).map(|q| q.id).collect()
}

/// Cor do grupo de equivalentes: a mesma para o mesmo parafuso, tirada das cores do tema.
fn cor_do_grupo(pilhas: &[Pilha], i: usize, v: &egui::Visuals) -> Color32 {
    let primeiro = pilhas.iter().position(|q| q.equivalente(&pilhas[i])).unwrap_or(i);
    let grupo = pilhas[..primeiro].iter().enumerate().filter(|(k, q)| pilhas.iter().position(|x| x.equivalente(q)) == Some(*k)).count();
    let base = [crate::theme::accent(v), crate::theme::st_ok(), crate::theme::st_warn(), v.hyperlink_color, v.text_color()];
    base[grupo % base.len()].gamma_multiply(if (grupo / base.len()) % 2 == 0 { 1. } else { 0.6 })
}

fn sinal(d: Option<&Diagnostico>) -> (&'static str, Color32) {
    match d.map(|x| x.nivel) {
        Some(Nivel::Erro) => ("✖", crate::theme::st_err()),
        Some(Nivel::Aviso) => ("⚠", crate::theme::st_warn()),
        _ => ("✔", crate::theme::st_ok()),
    }
}

enum Acao {
    Familia(Id, String),
    Tamanho(Id, String),
    Comprimento(Id, f64),
    Arruela(Id, bool),
    Porca(Id, bool),
    Opcao(usize, bool),
    Engajamento(f64),
    Mover(Id, V3),
    Apagar(Id),
    Recalcular,
}

fn combo(ui: &mut egui::Ui, id: (Id, &str), atual: String, itens: Vec<(String, String)>) -> Option<String> {
    let mut novo = None;
    egui::ComboBox::from_id_salt((id.0 .0, id.1)).width(84.).selected_text(atual.clone()).show_ui(ui, |ui| {
        for (v, n) in itens {
            if ui.selectable_label(n == atual, n).clicked() {
                novo = Some(v);
            }
        }
    });
    novo
}

fn tabela(ui: &mut egui::Ui, st: &AsmState, e: &mut Estado, acao: &mut Option<Acao>) {
    let parafusos: Vec<&Familia> = biblioteca::familias().iter().filter(|f| TipoFixador::e_parafuso(f.tipo)).collect();
    let v = ui.visuals().clone();
    egui::Grid::new("fix_tabela").striped(true).num_columns(8).show(ui, |ui| {
        for t in ["", "Ponto", "Família", "Tamanho", "Comprimento", "Arruela", "Porca", ""] {
            ui.label(RichText::new(tr(t)).small().strong());
        }
        ui.end_row();
        for (i, p) in st.asm.pilhas.iter().enumerate() {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12., 12.), Sense::hover());
            ui.painter().circle_filled(r.center(), 5., cor_do_grupo(&st.asm.pilhas, i, &v));
            let nome = if p.manual { format!("{} ({})", st.asm.name_of(p.ancora.comp), tr("manual")) } else { st.asm.name_of(p.ancora.comp) };
            if ui.selectable_label(e.sel == Some(p.id), nome).clicked() {
                e.sel = Some(p.id);
            }
            let fam = biblioteca::publica().familia(&p.familia);
            if let Some(n) = combo(ui, (p.id, "fam"), fam.map_or(p.familia.clone(), |f| f.norma.clone()), parafusos.iter().map(|f| (f.id.clone(), f.norma.clone())).collect()) {
                *acao = Some(Acao::Familia(p.id, n));
            }
            let tamanhos: Vec<(String, String)> = fam.map(|f| f.tamanhos.iter().map(|t| (t.designacao.clone(), t.designacao.clone())).collect()).unwrap_or_default();
            if let Some(n) = combo(ui, (p.id, "tam"), p.tamanho.clone(), tamanhos) {
                *acao = Some(Acao::Tamanho(p.id, n));
            }
            let serie: Vec<f64> = fam.and_then(|f| f.tamanho(&p.tamanho).map(|t| f.comprimentos_de(t))).unwrap_or_default();
            if let Some(n) = combo(ui, (p.id, "len"), fmt(p.comprimento), serie.iter().map(|l| (l.to_string(), fmt(*l))).collect()) {
                if let Ok(l) = n.parse() {
                    *acao = Some(Acao::Comprimento(p.id, l));
                }
            }
            let mut a = p.arruela;
            if ui.checkbox(&mut a, "").changed() {
                *acao = Some(Acao::Arruela(p.id, a));
            }
            let mut n = p.porca;
            if ui.checkbox(&mut n, "").changed() {
                *acao = Some(Acao::Porca(p.id, n));
            }
            let d = st.pilhas_diag.de(p.id);
            let (s, c) = sinal(d);
            let r = ui.label(RichText::new(s).color(c));
            if let Some(t) = d.map(|d| d.texto.as_str()).filter(|t| !t.is_empty()) {
                r.on_hover_text(t);
            }
            ui.end_row();
        }
    });
}

fn linha_aberta(ui: &mut egui::Ui, st: &AsmState, e: &Estado, acao: &mut Option<Acao>) {
    let Some(p) = e.sel.and_then(|id| st.asm.pilhas.iter().find(|p| p.id == id)) else { return };
    ui.separator();
    ui.label(RichText::new(format!("{} · {}", p.rotulo(), st.asm.name_of(p.ancora.comp))).strong());
    if let Some(d) = st.pilhas_diag.de(p.id).filter(|d| !d.texto.is_empty()) {
        ui.label(RichText::new(&d.texto).color(sinal(Some(d)).1));
    }
    if p.manual {
        let mut q = p.ancora.pos;
        let mut mudou = false;
        ui.horizontal(|ui| {
            for (k, l) in ["X", "Y", "Z"].into_iter().enumerate() {
                mudou |= ui.add(crate::ui_ext::dv(&mut q[k]).speed(0.5).prefix(format!("{l} ")).dica(tr("Mover o ponto move o prendedor."))).changed();
            }
        });
        if mudou {
            *acao = Some(Acao::Mover(p.id, q));
        }
    }
    if ui.button(tr("Apagar prendedor")).on_hover_text(tr("Tira o parafuso, as arruelas e a porca deste ponto.")).clicked() {
        *acao = Some(Acao::Apagar(p.id));
    }
}

fn opcoes(ui: &mut egui::Ui, st: &AsmState, acao: &mut Option<Acao>) {
    let auto: Vec<&Pilha> = st.asm.pilhas.iter().filter(|p| !p.manual).collect();
    let itens: [(&str, &str, fn(&Pilha) -> bool); 3] = [
        ("Seguir o diâmetro do furo", "O tamanho vem do furo: mudar o furo troca o parafuso.", |p| p.seguir_furo),
        ("Atualizar o comprimento", "Quando a pilha muda, o comprimento volta a sair da pega e da série.", |p| p.atualizar_comprimento),
        ("Trocar arruela e porca junto", "Arruela e porca acompanham o tamanho do parafuso.", |p| p.trocar_junto),
    ];
    for (k, (t, dica, ler)) in itens.into_iter().enumerate() {
        let mut v = !auto.is_empty() && auto.iter().all(|p| ler(p));
        if ui.checkbox(&mut v, tr(t)).on_hover_text(tr(dica)).changed() {
            *acao = Some(Acao::Opcao(k, v));
        }
    }
    let mut g = auto.first().map_or(fenkra_montagem::fixadores::ENGAJAMENTO_PADRAO, |p| p.engajamento);
    let r = ui.add(crate::ui_ext::dv(&mut g).speed(0.05).range(0.1..=5.).suffix(" × d").rotulo(tr("Engajamento mínimo de rosca")).dica(tr("Em furo roscado, quanto do parafuso precisa entrar na rosca, em múltiplos do diâmetro.")));
    if r.changed() {
        *acao = Some(Acao::Engajamento(g));
    }
}

/// Painel do assistente.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.fixadores.take() else { return };
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Assistente de prendedores").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 560., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    let (mut acao, mut fechar) = (None, false);
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        fechar = crate::app::done_bar(ui);
        if st.asm.pilhas.is_empty() {
            ui.label(RichText::new(tr("Nenhum prendedor nesta montagem: preencha os furos ou adicione um ponto.")).weak());
        }
        egui::ScrollArea::vertical().max_height(320.).auto_shrink([false, true]).show(ui, |ui| tabela(ui, st, &mut e, &mut acao));
        linha_aberta(ui, st, &e, &mut acao);
        ui.separator();
        ui.checkbox(&mut e.equivalentes, tr("Aplicar aos equivalentes")).on_hover_text(tr("A mudança vale para todas as linhas da mesma cor."));
        opcoes(ui, st, &mut acao);
        ui.horizontal(|ui| {
            if ui.button(tr("Recalcular")).on_hover_text(tr("Confere todos os pontos com os furos de agora.")).clicked() {
                acao = Some(Acao::Recalcular);
            }
            let t = if e.adicionando { tr("Clique numa face…") } else { tr("Adicionar ponto") };
            if ui.selectable_label(e.adicionando, t).on_hover_text(tr("Modo manual: o prendedor fica no ponto clicado e o automático não mexe nele.")).clicked() {
                e.adicionando = !e.adicionando;
            }
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    if fechar {
        st.fantasma_novo = true;
        return;
    }
    let mut erro = None;
    if let Some(a) = acao {
        erro = aplicar(st, &e, a);
    }
    st.fixadores = Some(e);
    if let Some(m) = erro {
        app.toast(m, true);
    }
}

fn aplicar(st: &mut AsmState, e: &Estado, a: Acao) -> Option<String> {
    let eq = e.equivalentes;
    let todas: Vec<Id> = st.asm.pilhas.iter().filter(|p| !p.manual).map(|p| p.id).collect();
    match a {
        Acao::Familia(id, f) => {
            let ids = alcance(&st.asm, id, eq);
            mudar(st, &ids, move |p| p.familia = f.clone());
        }
        Acao::Tamanho(id, t) => {
            let ids = alcance(&st.asm, id, eq);
            mudar(st, &ids, move |p| {
                p.tamanho = t.clone();
                p.seguir_furo = false;
            });
        }
        Acao::Comprimento(id, l) => {
            let ids = alcance(&st.asm, id, eq);
            mudar(st, &ids, move |p| {
                p.comprimento = l;
                p.atualizar_comprimento = false;
            });
        }
        Acao::Arruela(id, v) => {
            let ids = alcance(&st.asm, id, eq);
            mudar(st, &ids, move |p| p.arruela = v);
        }
        Acao::Porca(id, v) => {
            let ids = alcance(&st.asm, id, eq);
            mudar(st, &ids, move |p| p.porca = v);
        }
        Acao::Opcao(k, v) => mudar(st, &todas, move |p| match k {
            0 => p.seguir_furo = v,
            1 => p.atualizar_comprimento = v,
            _ => p.trocar_junto = v,
        }),
        Acao::Engajamento(g) => mudar(st, &todas, move |p| p.engajamento = g),
        Acao::Mover(id, q) => {
            let dir = st.dir()?;
            st.push_undo();
            if let Err(m) = pilhas::mover_ponto(&mut st.asm, &dir, biblioteca::publica(), id, q) {
                return Some(m);
            }
            st.solve();
        }
        Acao::Apagar(id) => {
            st.push_undo();
            pilhas::apagar(&mut st.asm, id);
            st.solve();
        }
        Acao::Recalcular => {
            atualizar(st);
        }
    }
    None
}

/// Cria o ponto manual na face clicada.
fn adicionar(st: &mut AsmState, e: &mut Estado, p: Pick) -> Result<(), String> {
    let Pick::Face { body, point, normal, .. } = p else { return Err("clique numa face".into()) };
    let comp = st.comp_de(body).ok_or("componente não encontrado")?;
    let pl = st.asm.node(comp).ok_or("componente não encontrado")?.placement;
    let dir = st.dir().ok_or("salve a montagem antes: as peças dos fixadores são gravadas na pasta dela")?;
    let local = pl.to_local(point.map(|v| v as f64));
    let n = pl.inverse().dir(normal.map(|v| -v as f64));
    let op = OpcoesFixadores { porca: false, ..Default::default() };
    st.push_undo();
    let id = pilhas::adicionar_manual(&mut st.asm, &dir, biblioteca::publica(), comp, local, n, &op.familia_parafuso.clone(), &e.novo.0, e.novo.1, &op, 0.)?;
    e.sel = Some(id);
    e.adicionando = false;
    st.solve();
    Ok(())
}

pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().and_then(|s| s.fixadores.as_ref()).is_some_and(|e| e.adicionando) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    let st = app.asm.as_mut().expect("montagem aberta");
    let Some(mut e) = st.fixadores.take() else { return true };
    let aviso = picks.into_iter().next_back().and_then(|p| adicionar(st, &mut e, p).err());
    st.fixadores = Some(e);
    if let Some(m) = aviso {
        app.toast(tr(&m).to_string(), true);
    }
    true
}

/// Marca de cada ponto na vista: a cor diz o estado, e o ponto com problema mostra o motivo.
pub fn overlay(app: &App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let Some(st) = app.asm.as_ref().filter(|s| s.fixadores.is_some()) else { return };
    let painter = ui.painter_at(rect);
    for p in &st.asm.pilhas {
        let Some(n) = st.asm.node(p.ancora.comp) else { continue };
        let q = n.placement.point(p.ancora.pos);
        let Some(s) = cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32)) else { continue };
        let d = st.pilhas_diag.de(p.id);
        let (_, c) = sinal(d);
        painter.circle_filled(s, 5., c);
        if let Some(d) = d.filter(|d| d.nivel == Nivel::Erro) {
            painter.text(s + egui::vec2(10., -10.), egui::Align2::LEFT_BOTTOM, &d.texto, egui::FontId::proportional(12.), c);
        }
    }
}

/// `FENKRA_DEMO_ASM=fixadores`: duas placas furadas, pilhas de dois tamanhos e um erro de pega.
pub fn demo(st: &mut AsmState) {
    let Some(dir) = st.dir() else { return };
    let ids: Vec<Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, fenkra_montagem::NodeKind::Group)).map(|n| n.id).collect();
    st.asm.node_mut(ids[1]).map(|n| n.placement.origin = [0., 0., 12.]);
    let mut op = OpcoesFixadores::default();
    op.folga = 1;
    let _ = fenkra_montagem::preencher_furos(&mut st.asm, Some(&dir), &mut st.cache, &op);
    let _ = pilhas::adicionar_manual(&mut st.asm, &dir, biblioteca::publica(), ids[1], [10., 10., 12.], [0., 0., -1.], "iso4762", "M5", 16., &OpcoesFixadores { porca: false, ..Default::default() }, 0.);
    st.solve();
    abrir(st, None);
    if let Some(p) = st.asm.pilhas.first().map(|p| p.id) {
        mudar(st, &[p], |p| {
            p.comprimento = 16.;
            p.atualizar_comprimento = false;
        });
        if let Some(e) = st.fixadores.as_mut() {
            e.sel = Some(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::{Assembly, NodeKind, Placement};
    use fenkra_core::{sketch::Curve, BoolOp, Document, Extent, Extrude, FeatureKind, Hole, HoleKind, OriginPlane, PlaneRef, Sketch, SketchData};
    use std::path::PathBuf;

    fn placa(d: f64, pontos: &[[f64; 2]]) -> Document {
        let mut doc = Document::new("Placa");
        let mut n = 7_400_000u64;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut base = SketchData::default();
        base.add_rect(&mut al, [-30., -30.], [30., 30.], false);
        let sb = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data: base }));
        doc.add_feature(FeatureKind::Extrude(Extrude { sketch: sb, op: BoolOp::Add, extent: Extent::Blind("8".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        let topo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "8");
        let mut c = SketchData::default();
        for q in pontos {
            let p = c.add_point(&mut al, q[0], q[1]);
            c.add_curve(&mut al, Curve::Point { p }, false);
        }
        let sc = doc.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(topo), data: c }));
        doc.add_feature(FeatureKind::Hole(Hole { sketch: sc, kind: HoleKind::Simples, diameter: d.to_string(), depth: Extent::ThroughAll, thread: None, thread_modeled: false, standard: None, points: vec![], dir: None, reverse: false, tip: None, taper: "0".into(), left_hand: false, extra: Default::default() }));
        doc
    }

    fn conjunto(nome: &str, pontos: &[[f64; 2]]) -> (AsmState, PathBuf) {
        let d = std::env::temp_dir().join(format!("fenkra_assist_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("a.fkp"), placa(6.6, pontos).to_json()).unwrap();
        let mut a = Assembly::new("t");
        a.add_component("A", NodeKind::Part { path: "a.fkp".into() }, Placement { origin: [0., 0., 8.], ..Default::default() }, None);
        a.add_component("B", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        let mut st = AsmState::new(a, Some(d.join("c.fka")));
        fenkra_montagem::preencher_furos(&mut st.asm, Some(&d), &mut st.cache, &OpcoesFixadores::default()).unwrap();
        (st, d)
    }

    #[test]
    fn mudar_o_furo_de_m6_para_m8_com_a_peca_salva_e_um_desfazer() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = conjunto("m8", &[[0., 0.]]);
        std::fs::write(d.join("a.fkp"), placa(9., &[[0., 0.]]).to_json()).unwrap();
        st.cache.forget(&d.join("a.fkp"));
        let aviso = apos_mudar_peca(&mut st).expect("avisa");
        assert!(aviso.contains('1'), "{aviso}");
        assert_eq!(st.asm.pilhas[0].tamanho, "M8");
        assert!(st.asm.pilhas[0].pecas.iter().all(|c| st.asm.name_of(*c).contains("M8")));
        assert_eq!(st.undo.len(), 1, "um desfazer");
        st.asm = st.undo.pop().unwrap();
        assert_eq!(st.asm.pilhas[0].tamanho, "M6");
        assert!(st.asm.pilhas[0].pecas.iter().all(|c| st.asm.name_of(*c).contains("M6")));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn trocar_o_tamanho_na_tabela_vale_para_os_equivalentes_com_um_desfazer() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = conjunto("tabela", &[[-15., 0.], [15., 0.]]);
        let id = st.asm.pilhas[0].id;
        let e = Estado { equivalentes: true, ..Default::default() };
        aplicar(&mut st, &e, Acao::Comprimento(id, 45.));
        assert_eq!(st.undo.len(), 1);
        assert!(st.asm.pilhas.iter().all(|p| p.comprimento == 45. && !p.atualizar_comprimento));
        // sem equivalentes, só a linha
        let e = Estado::default();
        aplicar(&mut st, &e, Acao::Tamanho(id, "M5".into()));
        assert_eq!(st.undo.len(), 2);
        let p = st.asm.pilhas.iter().find(|p| p.id == id).unwrap();
        assert_eq!(p.tamanho, "M5");
        assert!(!p.seguir_furo, "escolha à mão desliga o automático, senão o recálculo a desfaria");
        assert!(st.asm.pilhas.iter().any(|q| q.tamanho == "M6"));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn equivalentes_tem_a_mesma_cor_e_os_outros_nao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = conjunto("cor", &[[-15., 0.], [15., 0.], [0., 15.]]);
        let id = st.asm.pilhas[2].id;
        aplicar(&mut st, &Estado::default(), Acao::Comprimento(id, 45.));
        let v = egui::Visuals::dark();
        let c: Vec<Color32> = (0..3).map(|i| cor_do_grupo(&st.asm.pilhas, i, &v)).collect();
        assert_eq!(c[0], c[1]);
        assert_ne!(c[0], c[2]);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn apagar_na_tabela_tira_as_pecas_e_a_linha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d) = conjunto("apagar", &[[0., 0.]]);
        let id = st.asm.pilhas[0].id;
        aplicar(&mut st, &Estado::default(), Acao::Apagar(id));
        assert!(st.asm.pilhas.is_empty());
        assert_eq!(st.asm.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Part { .. })).count(), 2);
        let _ = std::fs::remove_dir_all(d);
    }
}
