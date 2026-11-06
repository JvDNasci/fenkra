// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel de interferências (B264): painel livre próprio, que não depende do painel de edição.
//!
//! A análise mora em `fenkra_montagem::interferencia`. Aqui ficam as opções, a lista e o realce:
//! escolher um item enquadra a câmera no volume comum e troca o que a cena mostra. As duas peças
//! ficam transparentes, o volume comum sai em vermelho e opaco, e as demais peças ficam em arame,
//! transparentes ou ocultas. O realce é um passe sobre a cena já pronta, então ligar e desligar não
//! recalcula nada, e a seleção da montagem continua a mesma por baixo.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::render::{LineVertex, MeshVertex, Scene};
use crate::ui_ext::dv;
use eframe::egui::{self, Rect, RichText};
use fenkra_montagem::interferencia::{self, Achado, Escopo, OpcoesInterf, Reparticao};
use glam::Vec3;

pub const PAINEL: &str = "interferencias";
/// O enquadramento mostra pelo menos isto em volta do volume comum (mm): um volume pequeno sozinho na
/// tela não diz onde ele fica.
const FOLGA_MIN: f32 = 30.;
/// Opacidade das duas peças do item escolhido e das demais, no modo transparente.
const ALFA_PECAS: f32 = 0.22;
const ALFA_DEMAIS: f32 = 0.08;

/// Como ficam as peças que não estão no item escolhido.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Demais {
    #[default]
    Arame,
    Transparentes,
    Ocultas,
}

#[derive(Default)]
pub struct Estado {
    pub op: OpcoesInterf,
    pub achados: Vec<Achado>,
    pub escolhido: Option<usize>,
    pub demais: Demais,
    /// as opções mudaram depois da última análise
    pub desatualizado: bool,
    pub motivo: String,
    pub rodou: bool,
}

/// Abre o painel e já analisa com as opções da última vez.
pub fn abrir(st: &mut AsmState) {
    let mut est = st.interf.take().unwrap_or_default();
    verificar(st, &mut est);
    st.interf = Some(est);
}

/// Refaz a análise. As peças escolhidas valem para os escopos que olham a seleção.
pub fn verificar(st: &mut AsmState, est: &mut Estado) {
    est.op.escolhidas = st.multi.iter().copied().chain(st.sel).collect();
    let dir = st.dir();
    est.achados = interferencia::analisar(&st.asm, dir.as_deref(), &mut st.cache, &est.op);
    est.escolhido = None;
    est.desatualizado = false;
    est.rodou = true;
    st.fantasma_novo = true;
}

/// Caixa para enquadrar o achado, com folga.
pub fn caixa_de(a: &Achado) -> Option<(Vec3, Vec3)> {
    let (lo, hi) = a.caixa?;
    let v = |p: [f64; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let (lo, hi) = (v(lo), v(hi));
    let folga = ((hi - lo).length() * 0.8).max(FOLGA_MIN);
    Some((lo - Vec3::splat(folga), hi + Vec3::splat(folga)))
}

fn titulo_do(a: &Achado) -> String {
    format!("{} × {}", a.a.nome, a.b.nome)
}

fn volume_do(a: &Achado) -> String {
    if a.coincidente() {
        tr("contato").to_string()
    } else {
        format!("{} mm³", fmt(a.volume))
    }
}

enum Acao {
    Verificar,
    Escolher(usize),
    Ignorar(usize),
    Restaurar(usize),
    Menores(f64),
    Fechar,
}

fn opcoes_ui(ui: &mut egui::Ui, est: &mut Estado) {
    let antes = est.op.clone();
    ui.horizontal(|ui| {
        ui.label(tr("Escopo"));
        let rotulo = |e: Escopo| match e {
            Escopo::Toda => tr("Toda a montagem"),
            Escopo::ContraAsDemais => tr("Escolhidas contra as demais"),
            Escopo::EntreEscolhidas => tr("Só entre as escolhidas"),
        };
        egui::ComboBox::from_id_salt("interf_escopo").selected_text(rotulo(est.op.escopo)).show_ui(ui, |ui| {
            for e in [Escopo::Toda, Escopo::ContraAsDemais, Escopo::EntreEscolhidas] {
                ui.selectable_value(&mut est.op.escopo, e, rotulo(e));
            }
        });
    })
    .response
    .on_hover_text(tr("Quais pares de peças entram na análise. Escolha as peças na árvore ou na vista."));
    ui.checkbox(&mut est.op.coincidencia, tr("Tratar coincidência como interferência")).on_hover_text(tr("Faces encostadas, sem volume comum, entram na lista como contato."));
    ui.checkbox(&mut est.op.sub_unico, tr("Subconjunto como peça única")).on_hover_text(tr("Ligado, o que há dentro de um subconjunto não é comparado entre si."));
    ui.add(dv(&mut est.op.tol).speed(0.01).range(0.0..=1e6).suffix(" mm³").rotulo(tr("Tolerância")).dica(tr("Volume comum abaixo disto é tratado como ruído e não aparece.")));
    if est.op != antes {
        est.desatualizado = true;
    }
}

fn lista_ativa(ui: &mut egui::Ui, est: &Estado, rep: &Reparticao, acao: &mut Option<Acao>) {
    let err = crate::theme::st_err();
    for &i in &rep.ativos {
        let a = &est.achados[i];
        let sel = est.escolhido == Some(i);
        ui.horizontal(|ui| {
            let r = ui.selectable_label(sel, RichText::new(format!("{}  {}", titulo_do(a), volume_do(a))).color(if a.coincidente() { crate::theme::st_warn() } else { err }));
            if r.on_hover_text(tr("Enquadra a interferência e mostra o volume comum em vermelho.")).clicked() {
                *acao = Some(Acao::Escolher(i));
            }
        });
    }
}

/// Painel, aberto até a pessoa fechar.
pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.interf.take() else { return };
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Interferências").to_string(), padrao: paineis::abaixo_do_cubo(area, 340.), largura: 340., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Direita);
    let rep = interferencia::separar(&est.achados, &st.asm.interf);
    let mut menores_que = st.asm.interf.menores_que;
    let mut acao = None;
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        if crate::app::done_bar(ui) {
            acao = Some(Acao::Fechar);
        }
        opcoes_ui(ui, &mut est);
        let antes = menores_que;
        ui.add(dv(&mut menores_que).speed(1.).range(0.0..=1e9).suffix(" mm³").rotulo(tr("Ignorar menores que")).dica(tr("Fica salvo na montagem. Os achados abaixo disto vão para a lista à parte.")));
        if menores_que != antes {
            acao = Some(Acao::Menores(menores_que));
        }
        ui.horizontal(|ui| {
            ui.label(tr("Demais peças"));
            for (d, t) in [(Demais::Arame, "Arame"), (Demais::Transparentes, "Transparentes"), (Demais::Ocultas, "Ocultas")] {
                if ui.selectable_label(est.demais == d, tr(t)).on_hover_text(tr("Como ficam as peças fora do item escolhido.")).clicked() {
                    est.demais = d;
                    st.fantasma_novo = true;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button(RichText::new(tr("Verificar")).strong()).clicked() {
                acao = Some(Acao::Verificar);
            }
            if est.desatualizado {
                ui.label(RichText::new(tr("opções mudaram: verifique de novo")).small().color(crate::theme::st_warn()));
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(360.).auto_shrink([false, true]).show(ui, |ui| {
            if est.rodou && rep.ativos.is_empty() {
                ui.label(RichText::new(tr("Nenhuma interferência entre os componentes")).color(crate::theme::st_ok()));
            }
            lista_ativa(ui, &est, &rep, &mut acao);
            if let Some(i) = est.escolhido.filter(|i| rep.ativos.contains(i)) {
                ui.add_space(4.);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut est.motivo).hint_text(tr("Motivo")).desired_width(170.));
                    if ui.button(tr("Ignorar")).on_hover_text(tr("Fica salvo na montagem, com o motivo, e vai para a lista de ignoradas.")).clicked() {
                        acao = Some(Acao::Ignorar(i));
                    }
                });
            }
            if !rep.ignorados.is_empty() {
                ui.add_space(6.);
                ui.label(RichText::new(format!("{} ({})", tr("Ignoradas"), rep.ignorados.len())).small().strong());
                for (i, motivo) in &rep.ignorados {
                    let a = &est.achados[*i];
                    ui.horizontal(|ui| {
                        let texto = if motivo.is_empty() { format!("{}  {}", titulo_do(a), volume_do(a)) } else { format!("{}  {} · {motivo}", titulo_do(a), volume_do(a)) };
                        if ui.selectable_label(est.escolhido == Some(*i), RichText::new(texto).small().weak()).clicked() {
                            acao = Some(Acao::Escolher(*i));
                        }
                        if ui.small_button(tr("Restaurar")).clicked() {
                            acao = Some(Acao::Restaurar(*i));
                        }
                    });
                }
            }
            if !rep.menores.is_empty() {
                ui.add_space(6.);
                ui.label(RichText::new(format!("{} ({})", tr("Menores que o limite"), rep.menores.len())).small().strong());
                for i in &rep.menores {
                    let a = &est.achados[*i];
                    if ui.selectable_label(est.escolhido == Some(*i), RichText::new(format!("{}  {}", titulo_do(a), volume_do(a))).small().weak()).clicked() {
                        acao = Some(Acao::Escolher(*i));
                    }
                }
            }
        });
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    let st = app.asm.as_mut().expect("montagem aberta");
    let mut caixa = None;
    match acao {
        Some(Acao::Fechar) => {
            st.fantasma_novo = true;
            return;
        }
        Some(Acao::Verificar) => verificar(st, &mut est),
        Some(Acao::Escolher(i)) => {
            if est.escolhido == Some(i) {
                est.escolhido = None;
            } else {
                est.escolhido = Some(i);
                caixa = est.achados.get(i).and_then(caixa_de);
            }
            st.fantasma_novo = true;
        }
        Some(Acao::Ignorar(i)) => ignorar(st, &mut est, i),
        Some(Acao::Restaurar(i)) => restaurar(st, &est, i),
        Some(Acao::Menores(v)) => {
            st.push_undo();
            st.asm.interf.menores_que = v;
        }
        None => {}
    }
    st.interf = Some(est);
    if let Some(b) = caixa {
        app.enquadrar_caixa(b);
    }
}

/// Grava o par na montagem, com um desfazer, e tira o realce.
pub fn ignorar(st: &mut AsmState, est: &mut Estado, i: usize) {
    let Some((a, b)) = est.achados.get(i).map(|x| (x.a.chave.clone(), x.b.chave.clone())) else { return };
    st.push_undo();
    st.asm.interf.ignorar(&a, &b, &est.motivo);
    est.motivo.clear();
    est.escolhido = None;
    st.fantasma_novo = true;
}

pub fn restaurar(st: &mut AsmState, est: &Estado, i: usize) {
    let Some((a, b)) = est.achados.get(i).map(|x| (x.a.chave.clone(), x.b.chave.clone())) else { return };
    st.push_undo();
    st.asm.interf.restaurar(&a, &b);
}

/// Refaz a cena para o item escolhido: as duas peças transparentes, o comum opaco em vermelho e as
/// demais como a pessoa pediu.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, s: &mut Scene) {
    let Some(st) = st else { return };
    let (Some(est), Some(vista)) = (st.interf.as_ref(), st.view.as_ref()) else { return };
    let Some(a) = est.escolhido.and_then(|i| est.achados.get(i)) else { return };
    realcar(a, est.demais, vista, [crate::theme::accent(v), crate::theme::st_ok(), crate::theme::st_err(), v.text_color()].map(|c| [c.r(), c.g(), c.b()]), s);
}

fn realcar(a: &Achado, demais: Demais, vista: &fenkra_solidos::RecomputeResult, cores: [[u8; 3]; 4], s: &mut Scene) {
    let [cor_a, cor_b, vermelho, neutra] = cores;
    let f = |c: [u8; 3]| [c[0] as f32 / 255., c[1] as f32 / 255., c[2] as f32 / 255., 1.];
    s.mesh.clear();
    s.indices.clear();
    s.edges.clear();
    s.lotes.clear();
    s.ghost.clear();
    for b in &vista.bodies {
        let lado = [(a.a.comp, cor_a), (a.b.comp, cor_b)].into_iter().find(|(c, _)| *c == b.collector).map(|x| x.1);
        match (lado, demais) {
            (Some(c), _) => {
                crate::previa::malha_fantasma(&mut s.ghost, &b.mesh, c, ALFA_PECAS);
                crate::scene::push_edges(&mut s.edges, &b.mesh, true, f(c));
            }
            (None, Demais::Arame) => crate::scene::push_edges(&mut s.edges, &b.mesh, true, f(neutra)),
            (None, Demais::Transparentes) => {
                crate::previa::malha_fantasma(&mut s.ghost, &b.mesh, neutra, ALFA_DEMAIS);
                crate::scene::push_edges(&mut s.edges, &b.mesh, true, f(neutra));
            }
            (None, Demais::Ocultas) => {}
        }
    }
    let m = &a.malha;
    let base = s.mesh.len() as u32;
    for (p, n) in m.positions.chunks_exact(3).zip(m.normals.chunks_exact(3)) {
        s.mesh.push(MeshVertex { pos: [p[0], p[1], p[2]], nrm: [n[0], n[1], n[2]], col: f(vermelho), mat: [0.6, 0., 1., 0.] });
    }
    s.indices.extend(m.indices.iter().map(|i| base + i));
    // o contato sem volume aparece por cima de tudo: é uma linha dentro de duas peças
    for linha in &a.contato {
        for par in linha.windows(2) {
            for p in par {
                s.overlay.push(LineVertex { pos: [p[0] as f32, p[1] as f32, p[2] as f32], col: f(vermelho) });
            }
        }
    }
}

/// `FENKRA_DEMO_ASM=interf[_oculta|_transparente]`: duas placas cruzadas e uma terceira ao lado, com o
/// primeiro item escolhido.
pub fn demo(st: &mut AsmState, nome: &str) {
    let ids: Vec<fenkra_core::Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, fenkra_montagem::NodeKind::Group)).map(|n| n.id).collect();
    if let Some(n) = st.asm.node_mut(ids[1]) {
        n.placement.origin = [30., 12., 6.];
    }
    st.asm.add_component("Placa ao lado", fenkra_montagem::NodeKind::Part { path: "placa.fkp".into() }, fenkra_montagem::Placement { origin: [110., 0., 0.], ..Default::default() }, None);
    st.asm.add_component("Placa de baixo", fenkra_montagem::NodeKind::Part { path: "placa.fkp".into() }, fenkra_montagem::Placement { origin: [0., 0., -10.], ..Default::default() }, None);
    st.dirty = true;
    abrir(st);
    if let Some(est) = st.interf.as_mut() {
        est.demais = match nome {
            "interf_oculta" => Demais::Ocultas,
            "interf_transparente" => Demais::Transparentes,
            _ => Demais::Arame,
        };
        est.escolhido = (!est.achados.is_empty()).then_some(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::Id;
    use fenkra_kernel::Mesh;
    use fenkra_montagem::interferencia::Lado;
    use fenkra_solidos::BodyResult;

    fn triangulo(x: f32) -> Mesh {
        Mesh { positions: vec![x, 0., 0., x + 1., 0., 0., x, 1., 0.], normals: vec![0., 0., 1., 0., 0., 1., 0., 0., 1.], indices: vec![0, 1, 2], face_ranges: vec![0], edges: vec![x, 0., 0., x + 1., 0., 0.], edge_ranges: vec![0], edge_smooth: vec![0], ..Default::default() }
    }

    fn vista() -> fenkra_solidos::RecomputeResult {
        let mut r = fenkra_solidos::recompute(&fenkra_core::Document::new("t"));
        let corpo = |id: u64, x: f32| BodyResult { collector: Id(id), color: [100, 100, 100], mesh: triangulo(x), mass: Default::default(), faces: 1, face_owner: vec![Id(id)], face_role: vec![-1] };
        r.bodies = vec![corpo(1, 0.), corpo(2, 5.), corpo(3, 10.)];
        r
    }

    fn achado() -> Achado {
        let lado = |id: u64| Lado { comp: Id(id), chave: id.to_string(), nome: format!("P{id}") };
        Achado { a: lado(1), b: lado(2), volume: 8., malha: triangulo(2.), contato: vec![], caixa: Some(([2., 0., 0.], [3., 1., 1.])) }
    }

    const CORES: [[u8; 3]; 4] = [[1, 2, 3], [4, 5, 6], [200, 0, 0], [9, 9, 9]];

    #[test]
    fn realce_troca_a_cena_inteira_e_deixa_o_comum_opaco() {
        let mut s = Scene::default();
        s.indices = vec![0, 0, 0];
        realcar(&achado(), Demais::Arame, &vista(), CORES, &mut s);
        assert_eq!(s.indices.len(), 3, "só o volume comum fica sólido");
        assert!(s.mesh.iter().all(|m| m.col[0] > 0.7 && m.col[1] == 0.), "e em vermelho");
        assert_eq!(s.ghost.len(), 6, "as duas peças transparentes");
        assert_eq!(s.edges.len(), 6, "as três peças deixam aresta: a terceira em arame");
    }

    #[test]
    fn demais_pecas_ocultas_nao_deixam_nada_e_transparentes_deixam_fantasma() {
        let mut s = Scene::default();
        realcar(&achado(), Demais::Ocultas, &vista(), CORES, &mut s);
        assert_eq!((s.ghost.len(), s.edges.len()), (6, 4));
        let mut s = Scene::default();
        realcar(&achado(), Demais::Transparentes, &vista(), CORES, &mut s);
        assert_eq!((s.ghost.len(), s.edges.len()), (9, 6));
    }

    #[test]
    fn enquadrar_mostra_folga_em_volta_de_um_volume_pequeno() {
        let (lo, hi) = caixa_de(&achado()).expect("caixa");
        assert!(hi.x - lo.x >= 2. * FOLGA_MIN);
    }

    #[test]
    fn ignorar_fica_na_montagem_com_um_desfazer_e_sai_da_lista_ativa() {
        let mut st = AsmState::new(fenkra_montagem::Assembly::new("t"), None);
        let mut est = Estado { achados: vec![achado()], motivo: "prensado".into(), escolhido: Some(0), ..Default::default() };
        ignorar(&mut st, &mut est, 0);
        assert_eq!(est.escolhido, None);
        assert_eq!(st.undo.len(), 1);
        assert!(st.modified, "ignorar pede salvar");
        let rep = interferencia::separar(&est.achados, &st.asm.interf);
        assert_eq!(rep.ignorados, vec![(0, "prensado".to_string())]);
        assert!(rep.ativos.is_empty());
        restaurar(&mut st, &est, 0);
        assert_eq!(interferencia::separar(&est.achados, &st.asm.interf).ativos, vec![0]);
    }
}
