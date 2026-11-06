// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando Chaveta (B292): escolhe o eixo e o cubo, mostra a chaveta fantasma dentro dos dois
//! rasgos e a insere restrita ao eixo.
//!
//! A conta é de `fenkra_montagem::chaveta` (pareamento dos rasgos, tabela, série de comprimentos e
//! as três restrições de plano); aqui ficam o painel, o fantasma e a alça de posição. Nada é gravado
//! em disco até apertar Inserir: a malha do fantasma sai do recálculo em memória do documento da
//! chaveta. A alça no centro da chaveta arrasta a posição ao longo do eixo, dentro do que cabe nos
//! dois rasgos.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_core::Id;
use fenkra_kernel::Mesh;
use fenkra_montagem::chaveta::{colocar, inserir_chaveta, parear, Colocacao, ParChaveta};
use fenkra_montagem::load::full_path;
use glam::Vec3;

pub const PAINEL: &str = "chaveta";
/// Opacidade da chaveta fantasma.
const ALFA: f32 = 0.55;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Papel {
    Eixo,
    Cubo,
}

/// Comando Chaveta aberto.
pub struct Estado {
    pub eixo: Option<Id>,
    pub cubo: Option<Id>,
    /// o próximo clique na vista escolhe este papel
    pub escolhendo: Option<Papel>,
    pub comprimento: Option<f64>,
    /// afastamento do centro da faixa comum, ao longo do eixo
    pub deslocamento: f64,
    /// pareamento do quadro: o que o painel, o fantasma e a inserção usam
    pub par: Option<Result<ParChaveta, String>>,
    /// malha da chaveta (diâmetro do eixo, comprimento) calculada em memória
    malha: Option<(f64, f64, Mesh)>,
}

impl Estado {
    pub fn novo() -> Self {
        Estado { eixo: None, cubo: None, escolhendo: None, comprimento: None, deslocamento: 0., par: None, malha: None }
    }
}

/// Que peça é esta: eixo escalonado (tem `d1`) ou cubo com furo (tem `furo`).
fn papel_de(st: &mut AsmState, id: Id) -> Option<Papel> {
    let dir = st.dir();
    let path = match st.asm.node(id).map(|n| &n.kind) {
        Some(fenkra_montagem::NodeKind::Part { path }) => path.clone(),
        _ => return None,
    };
    let doc = st.cache.documento(&full_path(dir.as_deref(), &path)).ok()?;
    let tem = |n: &str| doc.params.iter().any(|p| p.name == n);
    if tem("d1") {
        Some(Papel::Eixo)
    } else if tem("furo") {
        Some(Papel::Cubo)
    } else {
        None
    }
}

/// Abre o comando. Se já há peças escolhidas (na vista ou na árvore), elas ocupam os papéis pelo
/// que cada uma é.
pub fn abrir(st: &mut AsmState, picks: &[Pick]) {
    let mut est = Estado::novo();
    let mut comps = st.comps_of(picks);
    if let Some(s) = st.sel.filter(|s| !comps.contains(s)) {
        comps.push(s);
    }
    for c in comps {
        match papel_de(st, c) {
            Some(Papel::Eixo) if est.eixo.is_none() => est.eixo = Some(c),
            Some(Papel::Cubo) if est.cubo.is_none() => est.cubo = Some(c),
            _ => {}
        }
    }
    est.escolhendo = match (est.eixo, est.cubo) {
        (None, _) => Some(Papel::Eixo),
        (_, None) => Some(Papel::Cubo),
        _ => None,
    };
    st.sel = None;
    st.chaveta = Some(est);
}

fn fechar(st: &mut AsmState) {
    st.chaveta = None;
}

/// Pareia de novo, escolhe o comprimento válido e guarda a malha do fantasma.
fn atualizar(st: &mut AsmState, est: &mut Estado) {
    est.par = match (est.eixo, est.cubo) {
        (Some(e), Some(c)) => {
            let dir = st.dir();
            Some(parear(&st.asm, dir.as_deref(), &mut st.cache, e, c))
        }
        _ => None,
    };
    let Some(Ok(par)) = &est.par else { return };
    if est.comprimento.is_none_or(|l| !par.comprimentos.contains(&l)) {
        est.comprimento = par.comprimento_padrao();
    }
    let Some(l) = est.comprimento else { return };
    if est.malha.as_ref().is_some_and(|m| m.0 == par.diametro && m.1 == l) {
        return;
    }
    est.malha = fenkra_solidos::chaveta_documento(par.diametro, l)
        .ok()
        .and_then(|d| fenkra_solidos::recompute(&d).bodies.into_iter().next())
        .map(|b| (par.diametro, l, b.mesh));
}

/// Onde a chaveta fica agora, com o que a alça e o fantasma precisam.
fn colocacao<'a>(st: &AsmState, est: &'a Estado) -> Option<(Colocacao, &'a ParChaveta)> {
    let Some(Ok(par)) = &est.par else { return None };
    let col = colocar(&st.asm, par, est.comprimento?, est.deslocamento).ok()?;
    Some((col, par))
}

enum Acao {
    Fechar,
    Inserir,
    Escolher(Papel),
}

fn linha_peca(ui: &mut egui::Ui, st: &AsmState, est: &Estado, papel: Papel, acao: &mut Option<Acao>) {
    let (rotulo, dica, id) = match papel {
        Papel::Eixo => ("Eixo", "Peça com o rasgo de chaveta no trecho do eixo. Clique em Escolher e depois no eixo na vista.", est.eixo),
        Papel::Cubo => ("Cubo", "Peça com o furo e o rasgo de chaveta no cubo (engrenagem, polia). Clique em Escolher e depois no cubo na vista.", est.cubo),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(rotulo)).strong()).on_hover_text(tr(dica));
        let nome = id.map(|i| st.asm.name_of(i)).unwrap_or_else(|| tr("nenhum").to_string());
        ui.label(nome);
        let ativo = est.escolhendo == Some(papel);
        if ui.selectable_label(ativo, tr("Escolher")).on_hover_text(tr(dica)).clicked() {
            *acao = Some(Acao::Escolher(papel));
        }
    });
}

pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.chaveta.take() else { return };
    atualizar(st, &mut est);
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Chaveta").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 330., redimensiona: true };
    let posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    let mut acao = None;
    let v = ctx.global_style().visuals.clone();
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        if crate::app::done_bar(ui) {
            acao = Some(Acao::Fechar);
        }
        linha_peca(ui, st, &est, Papel::Eixo, &mut acao);
        linha_peca(ui, st, &est, Papel::Cubo, &mut acao);
        if let Some(p) = est.escolhendo {
            let t = if p == Papel::Eixo { "Clique no eixo na vista." } else { "Clique no cubo na vista." };
            ui.label(RichText::new(tr(t)).small().color(crate::theme::accent(&v)));
        }
        match &est.par {
            None => {
                ui.label(RichText::new(tr("Escolha o eixo e o cubo: o comando acha os dois rasgos e mostra a chaveta no lugar.")).weak().small());
            }
            Some(Err(m)) => {
                ui.label(RichText::new(format!("✖ {}", tr(m))).color(crate::theme::st_err()));
            }
            Some(Ok(par)) => {
                ui.separator();
                let t = par.tabela;
                ui.label(RichText::new(format!("Ø{}  ·  {} × {}  ·  t1 {}  ·  t2 {}", fmt(par.diametro), fmt(t.b), fmt(t.h), fmt(t.t1), fmt(t.t2))).strong()).on_hover_text(tr("Chaveta paralela forma A pela tabela do diâmetro do eixo: largura b, altura h, profundidade do rasgo no eixo t1 e no cubo t2."));
                if !par.comprimentos.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(tr("Comprimento")).on_hover_text(tr("Comprimentos da série que cabem na sobreposição dos dois rasgos. O maior já vem escolhido."));
                        let atual = est.comprimento.unwrap_or(0.);
                        egui::ComboBox::from_id_salt("chaveta_comprimento").selected_text(format!("{} mm", fmt(atual))).show_ui(ui, |ui| {
                            for l in &par.comprimentos {
                                if ui.selectable_label(*l == atual, format!("{} mm", fmt(*l))).clicked() {
                                    est.comprimento = Some(*l);
                                }
                            }
                        });
                    });
                    let folga = par.folga_de_posicao(est.comprimento.unwrap_or(0.));
                    ui.horizontal(|ui| {
                        ui.label(tr("Posição")).on_hover_text(tr("Afastamento do centro da chaveta em relação ao centro da faixa comum dos dois rasgos, ao longo do eixo. Também se arrasta a alça na vista."));
                        let dica = tr("Afasta a chaveta do centro da faixa comum dos rasgos, ao longo do eixo. Limitado ao que cabe nos dois.");
                        if folga > 1e-6 {
                            ui.add(crate::ui_ext::dv(&mut est.deslocamento).range(-folga..=folga).speed(0.2).suffix(" mm").dica(dica));
                        } else {
                            ui.label(RichText::new(tr("centralizada (sem folga)")).weak());
                            est.deslocamento = 0.;
                        }
                    });
                    ui.label(RichText::new(format!("{}: {} … {} mm", tr("Faixa comum dos rasgos (Z do eixo)"), fmt(par.z_ini), fmt(par.z_fim))).small().weak());
                }
                for a in &par.avisos {
                    let (sinal, cor) = if a.bloqueia { ("✖", crate::theme::st_err()) } else { ("⚠", crate::theme::st_warn()) };
                    ui.label(RichText::new(format!("{sinal} {}", tr(&a.texto))).small().color(cor));
                }
                ui.add_space(4.);
                let livre = par.bloqueio().is_none() && est.comprimento.is_some();
                let b = ui.add_enabled(livre, egui::Button::new(RichText::new(tr("Inserir chaveta")).strong()).min_size(egui::vec2(140., 28.)));
                if b.on_hover_text(tr("Grava a chaveta ao lado da montagem e a insere deitada no piso do rasgo do eixo, com contato e dois planos médios coincidentes: ela entra sem graus de liberdade.")).clicked() {
                    acao = Some(Acao::Inserir);
                }
                ui.label(RichText::new(tr("Nada é gravado em disco até você apertar Inserir chaveta.")).weak().small());
            }
        }
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    let mut aviso: Option<(String, bool)> = None;
    let st = app.asm.as_mut().expect("montagem aberta");
    match acao {
        Some(Acao::Fechar) => return,
        Some(Acao::Escolher(p)) => est.escolhendo = if est.escolhendo == Some(p) { None } else { Some(p) },
        Some(Acao::Inserir) => {
            if let (Some(e), Some(c)) = (est.eixo, est.cubo) {
                let antes = st.asm.clone();
                let dir = st.dir();
                match inserir_chaveta(&mut st.asm, dir.as_deref(), &mut st.cache, e, c, est.comprimento, est.deslocamento) {
                    Ok(r) => {
                        st.undo.push(antes);
                        st.redo.clear();
                        st.modified = true;
                        st.solve();
                        st.sel = Some(r.chaveta);
                        let msg = format!("{} {}×{}×{}", tr("Chaveta inserida:"), fmt(r.tabela.b), fmt(r.tabela.h), fmt(r.comprimento));
                        app.toast(msg, false);
                        return;
                    }
                    Err(m) => aviso = Some((m, true)),
                }
            }
        }
        None => {}
    }
    app.asm.as_mut().expect("montagem aberta").chaveta = Some(est);
    if let Some((m, erro)) = aviso {
        app.toast(m, erro);
    }
}

/// Depois da viewport: com o comando esperando uma peça, o clique escolhe o eixo ou o cubo.
pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().and_then(|s| s.chaveta.as_ref()).is_some_and(|e| e.escolhendo.is_some()) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    let st = app.asm.as_mut().expect("montagem aberta");
    let Some(mut est) = st.chaveta.take() else { return true };
    if let Some(c) = st.comps_of(&picks).into_iter().next_back() {
        match est.escolhendo {
            Some(Papel::Eixo) => est.eixo = Some(c),
            Some(Papel::Cubo) => est.cubo = Some(c),
            None => {}
        }
        est.deslocamento = 0.;
        // passa para o papel que ainda falta
        est.escolhendo = match (est.eixo, est.cubo) {
            (None, _) => Some(Papel::Eixo),
            (_, None) => Some(Papel::Cubo),
            _ => None,
        };
    }
    st.chaveta = Some(est);
    true
}

/// Chaveta fantasma na cena, na posição em que a inserção a põe.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, cena: &mut crate::render::Scene) {
    let Some(st) = st else { return };
    let Some(est) = st.chaveta.as_ref() else { return };
    let (Some((col, par)), Some((_, _, malha))) = (colocacao(st, est), est.malha.as_ref()) else { return };
    let cor: Color32 = if par.bloqueio().is_some() { crate::theme::st_err() } else { crate::theme::accent(v) };
    let t = fenkra_montagem::view::transformed(malha, &col.lugar);
    crate::previa::malha_fantasma(&mut cena.ghost, &t, [cor.r(), cor.g(), cor.b()], ALFA);
}

fn texto_com_fundo(p: &egui::Painter, ancora: Pos2, texto: String, cor: Color32, fundo: Color32) {
    let g = p.layout_no_wrap(texto, egui::FontId::proportional(12.), cor);
    p.rect_filled(Rect::from_min_size(ancora, g.size()).expand(2.), 3., fundo);
    p.galley(ancora, g, cor);
}

/// Legenda da chaveta e a alça que a desloca ao longo do eixo.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut est) = st.chaveta.take() else { return };
    let ja = colocacao(st, &est).map(|(c, p)| (c.lugar, c.comprimento, p.tabela, p.bloqueio().is_some(), p.folga_de_posicao(c.comprimento), st.asm.node(p.eixo).map(|n| n.placement)));
    if let Some((lugar, l, t, bloqueada, folga, Some(le))) = ja {
        let v = ui.visuals().clone();
        let cor = if bloqueada { crate::theme::st_err() } else { crate::theme::accent(&v) };
        let fundo = v.window_fill().gamma_multiply(0.85);
        let p = ui.painter_at(rect);
        let proj = |q: [f64; 3]| cam.project(rect, Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32));
        // eixo da peça no mundo: a alça anda ao longo dele
        let eixo = le.dir([0., 0., 1.]);
        let (centro, topo) = (lugar.point([0., 0., t.h]), lugar.point([0., 0., 0.]));
        if let (Some(c), Some(base)) = (proj(centro), proj(topo)) {
            texto_com_fundo(&p, c + Vec2::new(12., -22.), format!("{}×{}×{}", fmt(t.b), fmt(t.h), fmt(l)), cor, fundo);
            let mais = [centro[0] + eixo[0] * 10., centro[1] + eixo[1] * 10., centro[2] + eixo[2] * 10.];
            if let (Some(q), true) = (proj(mais), folga > 1e-6) {
                let na_tela = q - c;
                let px_por_mm = na_tela.length() / 10.;
                let id = egui::Id::new("alca_posicao_chaveta");
                let resp = ui.interact(Rect::from_center_size(c, egui::vec2(18., 18.)), id, egui::Sense::click_and_drag());
                let cor_alca = if resp.hovered() || resp.dragged() { crate::theme::st_warn() } else { crate::theme::st_ok() };
                p.line_segment([base, c], Stroke::new(1., cor.gamma_multiply(0.7)));
                p.circle_filled(c, 6.5, cor_alca);
                p.circle_stroke(c, 6.5, Stroke::new(1.5, v.window_stroke().color));
                let ctx = ui.ctx().clone();
                if resp.drag_started() {
                    ctx.data_mut(|d| d.insert_temp(id, est.deslocamento));
                }
                if resp.dragged() {
                    let alvo = crate::alca_cota::alvo_do_arrasto(ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(est.deslocamento), resp.drag_delta(), na_tela, px_por_mm);
                    ctx.data_mut(|d| d.insert_temp(id, alvo));
                    est.deslocamento = ((alvo * 2.).round() / 2.).clamp(-folga, folga);
                }
            }
        }
    }
    st.chaveta = Some(est);
}

/// `FENKRA_DEMO_ASM=chaveta`: eixo Ø25 com rasgo e engrenagem de furo Ø25 com rasgo no cubo, montados
/// no mesmo eixo, com o comando aberto. `chaveta_erro` põe o furo do cubo em Ø22, para o aviso aparecer.
pub fn demo(st: &mut AsmState, nome: &str) {
    use fenkra_montagem::{NodeKind, Placement};
    use fenkra_solidos::eixos::{eixo_documento, ParamEixo, RasgoChaveta, TrechoEixo};
    use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};
    let Some(dir) = st.dir() else { return };
    let eixo = ParamEixo {
        trechos: vec![
            TrechoEixo { diametro: 20., comprimento: 40., chanfro_fim: 0., rasgo: None },
            TrechoEixo { diametro: 25., comprimento: 80., chanfro_fim: 1., rasgo: Some(RasgoChaveta { comprimento: 50., desloc_inicio: 10. }) },
        ],
        furo_centro: false,
    };
    let furo = if nome == "chaveta_erro" { 22. } else { 25. };
    let cubo = ParamEngrenagem { furo, largura: 30., rasgo_cubo: true, dentes: 40, ..Default::default() };
    let (Ok(de), Ok(dc)) = (eixo_documento(&eixo), engrenagem_documento(&cubo)) else { return };
    let _ = std::fs::write(dir.join("eixo_demo.fkp"), de.to_json());
    let _ = std::fs::write(dir.join("cubo_demo.fkp"), dc.to_json());
    st.asm.nodes.retain(|n| matches!(n.kind, NodeKind::Group));
    st.asm.mates.clear();
    let e = st.asm.add_component("Eixo", NodeKind::Part { path: "eixo_demo.fkp".into() }, Placement::default(), None);
    if let Some(n) = st.asm.node_mut(e) {
        n.fixed = true;
    }
    let c = st.asm.add_component("Cubo", NodeKind::Part { path: "cubo_demo.fkp".into() }, Placement { origin: [0., 0., 55.], ..Default::default() }, None);
    st.dirty = true;
    st.fantasma_novo = true;
    let mut est = Estado::novo();
    est.eixo = Some(e);
    est.cubo = Some(c);
    st.chaveta = Some(est);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_montagem::{Assembly, NodeKind, Placement};
    use fenkra_solidos::eixos::{eixo_documento, ParamEixo, RasgoChaveta, TrechoEixo};
    use fenkra_solidos::engrenagens::{engrenagem_documento, ParamEngrenagem};

    fn conjunto(nome: &str) -> (AsmState, std::path::PathBuf, Id, Id) {
        let d = std::env::temp_dir().join(format!("fenkra_asm_chaveta_{nome}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let eixo = ParamEixo { trechos: vec![TrechoEixo { diametro: 25., comprimento: 90., chanfro_fim: 0., rasgo: Some(RasgoChaveta { comprimento: 50., desloc_inicio: 10. }) }], furo_centro: false };
        std::fs::write(d.join("eixo.fkp"), eixo_documento(&eixo).unwrap().to_json()).unwrap();
        let g = ParamEngrenagem { furo: 25., largura: 30., rasgo_cubo: true, dentes: 40, ..Default::default() };
        std::fs::write(d.join("cubo.fkp"), engrenagem_documento(&g).unwrap().to_json()).unwrap();
        let mut a = Assembly::new("t");
        let e = a.add_component("Eixo", NodeKind::Part { path: "eixo.fkp".into() }, Placement::default(), None);
        a.node_mut(e).unwrap().fixed = true;
        let c = a.add_component("Cubo", NodeKind::Part { path: "cubo.fkp".into() }, Placement { origin: [0., 0., 30.], ..Default::default() }, None);
        (AsmState::new(a, Some(d.join("m.fka"))), d, e, c)
    }

    #[test]
    fn abrir_com_duas_pecas_escolhe_os_papeis_pelo_que_cada_uma_e() {
        let (mut st, d, e, c) = conjunto("papeis");
        // o cubo veio primeiro na seleção: cada peça ocupa o papel que é
        st.sel = Some(c);
        abrir(&mut st, &[]);
        let est = st.chaveta.as_ref().unwrap();
        assert_eq!((est.eixo, est.cubo), (None, Some(c)));
        assert_eq!(est.escolhendo, Some(Papel::Eixo), "falta o eixo");
        // só o cubo escolhido: ainda pede o eixo; sem nada, também
        assert_eq!(papel_de(&mut st, e), Some(Papel::Eixo));
        assert_eq!(papel_de(&mut st, c), Some(Papel::Cubo));
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn atualizar_escolhe_o_maior_comprimento_e_a_malha_da_chaveta_fica_pronta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let (mut st, d, e, c) = conjunto("malha");
        let mut est = Estado::novo();
        est.eixo = Some(e);
        est.cubo = Some(c);
        atualizar(&mut st, &mut est);
        let par = est.par.as_ref().unwrap().as_ref().unwrap();
        assert!(par.bloqueio().is_none(), "{:?}", par.avisos);
        assert_eq!(est.comprimento, par.comprimento_padrao());
        let (_, l, malha) = est.malha.as_ref().expect("malha do fantasma");
        assert_eq!(Some(*l), est.comprimento);
        assert!(!malha.indices.is_empty());
        // o fantasma está onde a inserção a põe: o piso do rasgo, em x = 12,5 - 4
        st.chaveta = Some(est);
        let (col, _) = colocacao(&st, st.chaveta.as_ref().unwrap()).unwrap();
        assert!((col.lugar.origin[0] - 8.5).abs() < 1e-9, "{:?}", col.lugar.origin);
        // trocar o comprimento troca a malha
        let mut est = st.chaveta.take().unwrap();
        est.comprimento = Some(20.);
        atualizar(&mut st, &mut est);
        assert_eq!(est.malha.as_ref().unwrap().1, 20.);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn par_que_nao_casa_aparece_como_bloqueio_e_nao_insere() {
        let (mut st, d, e, c) = conjunto("bloqueio");
        st.asm.node_mut(c).unwrap().placement.origin = [4., 0., 30.];
        let mut est = Estado::novo();
        est.eixo = Some(e);
        est.cubo = Some(c);
        atualizar(&mut st, &mut est);
        let par = est.par.as_ref().unwrap().as_ref().unwrap();
        assert!(par.bloqueio().is_some_and(|b| b.contains("alinhado")));
        let dir = st.dir();
        let r = inserir_chaveta(&mut st.asm, dir.as_deref(), &mut st.cache, e, c, est.comprimento, 0.);
        assert!(r.is_err());
        assert_eq!(st.asm.nodes.len(), 2, "nada entrou");
        let _ = std::fs::remove_dir_all(d);
    }
}
