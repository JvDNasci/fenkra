// SPDX-License-Identifier: LGPL-2.1-or-later
//! Comando Repetição da montagem (B263). Nada entra na árvore antes de aceitar: o comando guarda um
//! rascunho da repetição e a vista mostra fantasmas das cópias, com uma marca em cada posição.
//!
//! A direção não nasce pronta. Clicar numa aresta, eixo ou face dá a direção (ou o eixo, no circular)
//! e o espaçamento começa no tamanho da peça naquela direção, para as cópias não nascerem umas
//! dentro das outras. No modo Pelos furos, clicar numa peça segue um padrão de furos dela.
//!
//! Na vista, a seta na primeira cópia arrasta o espaçamento e clicar numa marca pula a cópia dela.
//! As contas ficam em `fenkra_montagem::repeticao`.

use crate::app::{fmt, App};
use crate::asm_ui::AsmState;
use crate::i18n::tr;
use crate::paineis::{self, Encaixe, Painel};
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke};
use fenkra_core::Id;
use fenkra_montagem::load::full_path;
use fenkra_montagem::repeticao::{self, PadraoFuros, Passo};
use fenkra_montagem::{view, Assembly, CompPattern, Geom, MateRef, NodeKind, PatternKind, V3};
use glam::Vec3;

pub const PAINEL: &str = "repeticao";
/// Raio da marca clicável de cada posição, em pontos.
const RAIO_MARCA: f32 = 8.;
/// Componente vazio do rascunho: ainda não há o que repetir.
const NENHUM: Id = Id(0);

/// O que o próximo clique na vista escolhe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alvo {
    Componentes,
    Direcao,
    Direcao2,
}

pub struct Estado {
    pub r: CompPattern,
    pub alvo: Alvo,
    /// repetição em edição; `antes` é a montagem como estava (Cancelar volta a ela)
    pub editando: Option<Id>,
    pub antes: Option<Assembly>,
    /// padrões de furos da peça clicada, no modo Pelos furos
    pub padroes: Vec<PadraoFuros>,
    /// nome do que deu cada direção, para as caixas do painel
    pub nomes: [String; 2],
}

fn linear_vazio() -> PatternKind {
    PatternKind::Linear { dir: [0.; 3], count: "3".into(), spacing: "0".into(), dir2: None, count2: "2".into(), spacing2: "0".into() }
}

fn circular_vazio() -> PatternKind {
    PatternKind::Circular { origin: [0.; 3], axis: [0.; 3], count: "4".into(), angle: "360".into() }
}

impl Estado {
    fn novo(comps: &[Id]) -> Self {
        let mut r = CompPattern::rascunho(comps.first().copied().unwrap_or(NENHUM), linear_vazio());
        r.extras = comps.iter().skip(1).copied().collect();
        let alvo = if comps.is_empty() { Alvo::Componentes } else { Alvo::Direcao };
        Estado { r, alvo, editando: None, antes: None, padroes: vec![], nomes: Default::default() }
    }

    /// Há componente e direção: dá para mostrar as cópias e aceitar.
    pub fn pronto(&self) -> bool {
        self.r.source != NENHUM
            && match &self.r.kind {
                PatternKind::Linear { dir, .. } => self.r.ref1.is_some() || dir.iter().any(|x| x.abs() > 1e-12),
                PatternKind::Circular { axis, .. } => self.r.ref1.is_some() || axis.iter().any(|x| x.abs() > 1e-12),
                PatternKind::Furos { pontos, .. } => pontos.len() > 1,
            }
    }

    fn comps(&self) -> Vec<Id> {
        std::iter::once(self.r.source).filter(|s| *s != NENHUM).chain(self.r.extras.iter().copied()).collect()
    }

    /// Soma ou tira um componente; o primeiro é o original.
    fn alternar(&mut self, c: Id) {
        let mut v = self.comps();
        match v.iter().position(|x| *x == c) {
            Some(i) => {
                v.remove(i);
            }
            None => v.push(c),
        }
        self.r.source = v.first().copied().unwrap_or(NENHUM);
        self.r.extras = v.into_iter().skip(1).collect();
    }
}

/// Abre o comando com os componentes escolhidos. Grupo e repetição não contam.
pub fn abrir(st: &mut AsmState) {
    let comps: Vec<Id> = st.sel.into_iter().chain(st.multi.iter().copied()).filter(|s| st.asm.node(*s).is_some_and(|n| !matches!(n.kind, NodeKind::Group))).fold(vec![], |mut v, id| {
        if !v.contains(&id) {
            v.push(id);
        }
        v
    });
    st.repeticao = Some(Estado::novo(&comps));
    st.fantasma_novo = true;
}

/// Reabre uma repetição: as cópias saem da vista enquanto o rascunho manda.
pub fn editar(st: &mut AsmState, id: Id) {
    let Some(p) = st.asm.patterns.iter().find(|p| p.id == id).cloned() else { return };
    let antes = st.asm.clone();
    if let Some(q) = st.asm.patterns.iter_mut().find(|q| q.id == id) {
        q.pular = vec![];
        q.kind = match &q.kind {
            PatternKind::Linear { dir, spacing, .. } => PatternKind::Linear { dir: *dir, count: "1".into(), spacing: spacing.clone(), dir2: None, count2: "1".into(), spacing2: "0".into() },
            PatternKind::Circular { origin, axis, angle, .. } => PatternKind::Circular { origin: *origin, axis: *axis, count: "1".into(), angle: angle.clone() },
            PatternKind::Furos { comp, feature, .. } => PatternKind::Furos { comp: *comp, feature: *feature, pontos: vec![] },
        };
    }
    st.asm.update_patterns();
    st.dirty = true;
    let padroes = match &p.kind {
        PatternKind::Furos { comp, .. } => padroes_da_peca(st, *comp),
        _ => vec![],
    };
    let nome = |st: &AsmState, r: &Option<MateRef>| r.as_ref().map(|r| st.asm.name_of(r.comp)).unwrap_or_default();
    let nomes = [nome(st, &p.ref1), nome(st, &p.ref2)];
    st.repeticao = Some(Estado { r: p, alvo: Alvo::Direcao, editando: Some(id), antes: Some(antes), padroes, nomes });
}

fn padroes_da_peca(st: &mut AsmState, comp: Id) -> Vec<PadraoFuros> {
    let dir = st.dir();
    let Some(NodeKind::Part { path }) = st.asm.node(comp).map(|n| n.kind.clone()) else { return vec![] };
    st.cache.get(&full_path(dir.as_deref(), &path), false).map(|lp| repeticao::padroes_de_furos(&lp.doc)).unwrap_or_default()
}

/// Desiste: a montagem volta como estava.
pub fn cancelar(st: &mut AsmState) {
    if let Some(e) = st.repeticao.take() {
        if let Some(a) = e.antes {
            st.asm = a;
            st.dirty = true;
        }
    }
    st.fantasma_novo = true;
}

/// Grava a repetição, com um desfazer.
pub fn aceitar(st: &mut AsmState) -> Option<Id> {
    let e = st.repeticao.take()?;
    if !e.pronto() {
        st.repeticao = Some(e);
        return None;
    }
    let mut r = e.r;
    if let Some(a) = e.antes {
        // volta às cópias de antes: elas continuam as mesmas, com as restrições que tiverem
        st.asm = a;
    }
    // a direção que a referência dava fica guardada: se a referência sumir, a repetição não para
    fixar_direcao(&st.asm, &mut r);
    st.push_undo();
    let id = st.asm.gravar_repeticao(&r, e.editando);
    st.sel = Some(id);
    st.dirty = true;
    st.fantasma_novo = true;
    Some(id)
}

fn fixar_direcao(a: &Assembly, r: &mut CompPattern) {
    let vars = a.vars();
    let (mut r1, mut r2) = (r.clone(), r.clone());
    r1.inverter = false;
    r2.inverter2 = false;
    let d = a.direcoes(&r1, &vars);
    let d2 = a.direcoes(&r2, &vars);
    match &mut r.kind {
        PatternKind::Linear { dir, dir2, .. } => {
            *dir = d.0;
            if dir2.is_some() {
                *dir2 = Some(d2.1);
            }
        }
        PatternKind::Circular { axis, .. } => *axis = d.0,
        PatternKind::Furos { .. } => {}
    }
}

/// Tamanho da peça ao longo de `d`: o espaçamento inicial, com as cópias lado a lado.
fn tamanho_ao_longo(st: &AsmState, comps: &[Id], d: V3) -> Option<f64> {
    let vista = st.view.as_ref()?;
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for b in vista.bodies.iter().filter(|b| comps.contains(&b.collector)) {
        for p in b.mesh.positions.chunks_exact(3) {
            let x = p[0] as f64 * d[0] + p[1] as f64 * d[1] + p[2] as f64 * d[2];
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    (hi > lo).then_some(hi - lo)
}

/// Sentido da referência, para o espaçamento inicial.
fn sentido_de(st: &AsmState, r: &MateRef) -> Option<V3> {
    let pl = st.asm.node(r.comp).map(|n| n.placement).unwrap_or_default();
    match r.geom {
        Geom::Plane { n, .. } => Some(pl.dir(n)),
        Geom::Axis { d, .. } => Some(pl.dir(d)),
        Geom::Point(_) => None,
    }
}

/// Um clique na vista, conforme o alvo da vez.
fn receber(st: &mut AsmState, e: &mut Estado, p: Pick) -> Result<(), String> {
    let furos = matches!(e.r.kind, PatternKind::Furos { .. });
    if e.alvo == Alvo::Componentes && !furos {
        let (Pick::Face { body, .. } | Pick::Edge { body, .. } | Pick::Vertex { body, .. }) = p else { return Ok(()) };
        if let Some(c) = st.comp_de(body) {
            e.alternar(c);
        }
        return Ok(());
    }
    let r = st.mate_ref(p)?;
    if furos {
        let padroes = padroes_da_peca(st, r.comp);
        let Some(f) = padroes.first().cloned() else { return Err("esta peça não tem padrão de furos".into()) };
        e.r.kind = PatternKind::Furos { comp: r.comp, feature: f.feature, pontos: f.pontos };
        e.nomes[0] = st.asm.name_of(r.comp);
        e.padroes = padroes;
        return Ok(());
    }
    if matches!(r.geom, Geom::Point(_)) {
        return Err("escolha uma aresta, um eixo ou uma face".into());
    }
    let d = sentido_de(st, &r).unwrap_or([1., 0., 0.]);
    let tam = tamanho_ao_longo(st, &e.comps(), d).filter(|t| *t > 1e-6);
    let nome = st.asm.name_of(r.comp);
    if e.alvo == Alvo::Direcao2 {
        e.nomes[1] = nome;
        e.r.ref2 = Some(r);
        if let PatternKind::Linear { dir2, spacing2, .. } = &mut e.r.kind {
            *dir2 = Some(d);
            if let Some(t) = tam.filter(|_| spacing2 == "0") {
                *spacing2 = fmt_num(t);
            }
        }
        return Ok(());
    }
    e.nomes[0] = nome;
    e.r.ref1 = Some(r);
    if let PatternKind::Linear { dir, spacing, .. } = &mut e.r.kind {
        *dir = d;
        if let Some(t) = tam.filter(|_| spacing == "0") {
            *spacing = fmt_num(t);
        }
    }
    Ok(())
}

fn fmt_num(v: f64) -> String {
    let s = format!("{:.3}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().is_some_and(|s| s.repeticao.is_some()) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    let st = app.asm.as_mut().expect("montagem aberta");
    let Some(mut e) = st.repeticao.take() else { return true };
    let mut aviso = None;
    for p in picks {
        if let Err(m) = receber(st, &mut e, p) {
            aviso = Some(m);
        }
        st.fantasma_novo = true;
    }
    st.repeticao = Some(e);
    if let Some(m) = aviso {
        app.toast(tr(&m).to_string(), true);
    }
    true
}

pub fn teclas(app: &mut App, ctx: &egui::Context) {
    if !app.asm.as_ref().is_some_and(|s| s.repeticao.is_some()) || ctx.egui_wants_keyboard_input() {
        return;
    }
    let (enter, tab, esc) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Tab), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)));
    let st = app.asm.as_mut().expect("montagem aberta");
    if esc {
        cancelar(st);
    } else if enter {
        aceitar(st);
    } else if tab {
        if let Some(e) = st.repeticao.as_mut() {
            e.r.inverter = !e.r.inverter;
            st.fantasma_novo = true;
        }
    }
}

/// Caixa de seleção: mostra o que foi escolhido e, ao clicar, recebe o próximo clique da vista.
fn caixa(ui: &mut egui::Ui, e: &mut Estado, alvo: Alvo, rotulo: &str, texto: String, dica: &str) {
    ui.horizontal(|ui| {
        ui.label(tr(rotulo));
        let ativa = e.alvo == alvo;
        let cor = if ativa { crate::theme::accent(ui.visuals()) } else { ui.visuals().weak_text_color() };
        let b = egui::Button::new(RichText::new(texto).color(cor)).stroke(Stroke::new(if ativa { 1.5 } else { 0.5 }, cor)).min_size(egui::vec2(170., 22.));
        if ui.add(b).on_hover_text(tr(dica)).clicked() {
            e.alvo = alvo;
        }
    });
}

fn conteudo(ui: &mut egui::Ui, st: &AsmState, e: &mut Estado, vars: &crate::editor::Vars) -> bool {
    let antes = e.r.clone();
    let modo = match e.r.kind {
        PatternKind::Linear { .. } => 0,
        PatternKind::Circular { .. } => 1,
        PatternKind::Furos { .. } => 2,
    };
    let mut novo = modo;
    ui.horizontal(|ui| {
        ui.selectable_value(&mut novo, 0, tr("Linear"));
        ui.selectable_value(&mut novo, 1, tr("Circular"));
        ui.selectable_value(&mut novo, 2, tr("Pelos furos")).on_hover_text(tr("Segue um padrão de furos de uma peça e respeita o que ele pulou."));
    });
    if novo != modo {
        e.r.kind = match novo {
            0 => linear_vazio(),
            1 => circular_vazio(),
            _ => PatternKind::Furos { comp: NENHUM, feature: NENHUM, pontos: vec![] },
        };
        e.r.ref1 = None;
        e.r.ref2 = None;
        e.r.pular.clear();
        e.nomes = Default::default();
        e.alvo = if e.r.source == NENHUM { Alvo::Componentes } else { Alvo::Direcao };
    }
    let nomes: Vec<String> = e.comps().iter().map(|c| st.asm.name_of(*c)).collect();
    let texto = if nomes.is_empty() { tr("clique nas peças").to_string() } else { nomes.join(", ") };
    caixa(ui, e, Alvo::Componentes, "Componentes", texto, "Com esta caixa ativa, clicar numa peça soma ou tira da repetição.");
    let vazio = |n: &str, t: &str| if n.is_empty() { tr(t).to_string() } else { n.to_string() };
    match &e.r.kind {
        PatternKind::Linear { .. } => {
            let t = vazio(&e.nomes[0], "clique em aresta, eixo ou face");
            caixa(ui, e, Alvo::Direcao, "Direção", t, "Aresta, eixo ou face de uma peça. Mover ou mudar a peça atualiza a direção.");
        }
        PatternKind::Circular { .. } => {
            let t = vazio(&e.nomes[0], "clique em eixo ou aresta circular");
            caixa(ui, e, Alvo::Direcao, "Eixo", t, "Eixo de furo, aresta circular ou face. Mover ou mudar a peça atualiza o eixo.");
        }
        PatternKind::Furos { .. } => {
            let t = vazio(&e.nomes[0], "clique na peça com os furos");
            caixa(ui, e, Alvo::Direcao, "Peça dos furos", t, "A peça cujo padrão de furos as cópias seguem.");
        }
    }
    match &mut e.r.kind {
        PatternKind::Linear { count, spacing, dir2, count2, spacing2, .. } => {
            ui.checkbox(&mut e.r.inverter, tr("Inverter")).on_hover_text(tr("Troca o sentido da direção. Atalho: Tab."));
            crate::editor::expr(ui, "Quantidade", count, vars, "");
            crate::editor::expr(ui, "Espaçamento", spacing, vars, "mm");
            let mut segunda = dir2.is_some();
            if ui.checkbox(&mut segunda, tr("Segunda direção")).on_hover_text(tr("Faz uma grade: repete também em outra direção.")).changed() {
                *dir2 = segunda.then_some([0.; 3]);
                if !segunda {
                    e.r.ref2 = None;
                    e.nomes[1].clear();
                } else {
                    e.alvo = Alvo::Direcao2;
                }
            }
            if dir2.is_some() {
                crate::editor::expr(ui, "Quantidade", count2, vars, "");
                crate::editor::expr(ui, "Espaçamento", spacing2, vars, "mm");
                ui.checkbox(&mut e.r.inverter2, tr("Inverter segunda direção"));
                let t = vazio(&e.nomes[1], "clique em aresta, eixo ou face");
                caixa(ui, e, Alvo::Direcao2, "Direção 2", t, "Aresta, eixo ou face da segunda direção.");
            }
        }
        PatternKind::Circular { count, angle, .. } => {
            ui.checkbox(&mut e.r.inverter, tr("Inverter")).on_hover_text(tr("Troca o sentido do giro. Atalho: Tab."));
            crate::editor::expr(ui, "Quantidade", count, vars, "");
            crate::editor::expr(ui, "Ângulo total", angle, vars, "°");
        }
        PatternKind::Furos { feature, pontos, .. } => {
            if e.padroes.len() > 1 {
                let atual = e.padroes.iter().find(|p| p.feature == *feature).map(|p| p.nome.clone()).unwrap_or_default();
                egui::ComboBox::from_id_salt("rep_padrao").selected_text(atual).show_ui(ui, |ui| {
                    for p in &e.padroes {
                        if ui.selectable_label(p.feature == *feature, &p.nome).clicked() {
                            *feature = p.feature;
                            *pontos = p.pontos.clone();
                        }
                    }
                });
            }
            ui.label(RichText::new(format!("{} {}", pontos.len(), tr("furos no padrão"))).small().weak());
        }
    }
    if !e.r.pular.is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} {}", e.r.pular.len(), tr("cópia(s) pulada(s)"))).small());
            if ui.small_button(tr("Restaurar")).clicked() {
                e.r.pular.clear();
            }
        });
    }
    ui.label(RichText::new(tr("Na vista: arraste a seta para o espaçamento; clique numa marca para pular a cópia. Enter aceita, Esc cancela.")).small().weak());
    e.r != antes
}

pub fn painel(app: &mut App, ctx: &egui::Context, area: Rect) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.repeticao.take() else { return };
    let cfg = Painel { nome: PAINEL.into(), titulo: tr("Repetição").to_string(), padrao: paineis::ao_lado_da_arvore(area), largura: 300., redimensiona: false };
    let posicao = app.settings.painel(PAINEL, Encaixe::Esquerda);
    let vars: crate::editor::Vars = st.asm.vars().into_iter().map(|(k, v)| (k, v)).collect();
    let (mut ok, mut nao, mut mudou) = (false, false, false);
    let pronto = e.pronto();
    let saida = paineis::mostrar(ctx, area, &cfg, posicao, |ui| {
        ui.horizontal(|ui| {
            ok = ui.add_enabled(pronto, egui::Button::new(RichText::new(tr("✔ Aceitar")).strong())).clicked();
            nao = ui.button(tr("Cancelar")).clicked();
        });
        ui.separator();
        mudou = conteudo(ui, st, &mut e, &vars);
    });
    if let Some(p) = saida.movido {
        app.settings.guardar_painel(PAINEL, p);
    }
    st.repeticao = Some(e);
    if mudou {
        st.fantasma_novo = true;
    }
    if nao {
        cancelar(st);
    } else if ok {
        aceitar(st);
    }
}

/// Fantasmas das cópias, na cor de destaque.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, cena: &mut crate::render::Scene) {
    let Some(st) = st else { return };
    let (Some(e), Some(vista)) = (st.repeticao.as_ref().filter(|e| e.pronto()), st.view.as_ref()) else { return };
    let c = crate::theme::accent(v);
    let vars = st.asm.vars();
    for p in st.asm.passos(&e.r, &vars).iter().filter(|p| !p.pulada) {
        for id in e.comps() {
            let Some(pl) = st.asm.node(id).map(|n| n.placement) else { continue };
            let delta = p.aplicar(&pl).then(&pl.inverse());
            for b in vista.bodies.iter().filter(|b| b.collector == id) {
                let m = view::transformed(&b.mesh, &delta);
                for fi in 0..m.face_ranges.len() {
                    crate::previa::face_fantasma(&mut cena.ghost, &m, fi, [c.r(), c.g(), c.b()]);
                }
            }
        }
    }
}

fn v3(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Centro das peças repetidas, levado até a posição.
fn centro_no_passo(p: &Passo, c: V3) -> V3 {
    let pl = fenkra_montagem::Placement { origin: c, ..Default::default() };
    p.aplicar(&pl).origin
}

/// Novo espaçamento depois de arrastar a seta `px` pontos, com `px_mm` pontos por milímetro.
fn espacamento_arrastado(atual: f64, px: f32, px_mm: f32) -> f64 {
    let v = atual + (px / px_mm.max(1e-3)) as f64;
    (v * 10.).round() / 10.
}

/// Alterna a posição na lista de puladas.
fn alternar_pulada(r: &mut CompPattern, chave: (u32, u32)) {
    match r.pular.iter().position(|k| *k == chave) {
        Some(i) => {
            r.pular.remove(i);
        }
        None => r.pular.push(chave),
    }
}

/// Marcas e seta na vista. Devolve se mudou o rascunho. `vista` é o resultado que a viewport mostra:
/// enquanto ela desenha, ele está em `App::result`, não em `AsmState::view`.
fn quadro(st: &AsmState, vista: Option<&fenkra_solidos::RecomputeResult>, e: &mut Estado, ui: &mut egui::Ui, rect: Rect, cam: &Camera) -> bool {
    if !e.pronto() {
        return false;
    }
    let comps = e.comps();
    let centro = vista.and_then(|r| view::centro_caixa(r.bodies.iter().filter(|b| comps.contains(&b.collector)).map(|b| &b.mesh))).or_else(|| st.asm.node(e.r.source).map(|n| n.placement.origin));
    let Some(centro) = centro else { return false };
    let vars = st.asm.vars();
    let passos = st.asm.passos(&e.r, &vars);
    let v = ui.visuals().clone();
    let (cor, erro) = (crate::theme::accent(&v), crate::theme::st_warn());
    let painter = ui.painter_at(rect);
    let mut mudou = false;
    for p in &passos {
        let Some(s) = cam.project(rect, v3(centro_no_passo(p, centro))) else { continue };
        let id = egui::Id::new(("rep_marca", p.chave));
        let resp = ui.interact(Rect::from_center_size(s, egui::vec2(RAIO_MARCA * 2.4, RAIO_MARCA * 2.4)), id, Sense::click());
        let quente = resp.hovered();
        if p.pulada {
            painter.circle_stroke(s, RAIO_MARCA, Stroke::new(1.5, erro));
            let d = RAIO_MARCA * 0.6;
            painter.line_segment([s + egui::vec2(-d, -d), s + egui::vec2(d, d)], Stroke::new(1.5, erro));
            painter.line_segment([s + egui::vec2(-d, d), s + egui::vec2(d, -d)], Stroke::new(1.5, erro));
        } else {
            painter.circle_filled(s, if quente { RAIO_MARCA } else { RAIO_MARCA - 2. }, cor);
        }
        if resp.on_hover_text(tr(if p.pulada { "Clique para trazer a cópia de volta" } else { "Clique para pular esta cópia" })).clicked() {
            alternar_pulada(&mut e.r, p.chave);
            mudou = true;
        }
    }
    mudou |= seta(st, e, &passos, centro, ui, rect, cam, cor);
    mudou
}

/// Seta do espaçamento, depois da primeira cópia.
#[allow(clippy::too_many_arguments)]
fn seta(st: &AsmState, e: &mut Estado, passos: &[Passo], centro: V3, ui: &mut egui::Ui, rect: Rect, cam: &Camera, cor: Color32) -> bool {
    let PatternKind::Linear { spacing, .. } = &mut e.r.kind else { return false };
    let vars = st.asm.vars();
    let Ok(atual) = fenkra_core::expr::eval_str(spacing, &vars) else { return false };
    let Some(p1) = passos.iter().find(|p| p.chave == (1, 0)) else { return false };
    let u = p1.desloc;
    let tam = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    if tam < 1e-9 {
        return false;
    }
    let base = centro_no_passo(p1, centro);
    let ponta = [0, 1, 2].map(|i| base[i] + u[i] / tam * 10.);
    let (Some(a), Some(b)) = (cam.project(rect, v3(base)), cam.project(rect, v3(ponta))) else { return false };
    let na_tela = b - a;
    let px_mm = na_tela.length() / 10.;
    if px_mm < 1e-3 {
        return false;
    }
    let dir = na_tela / na_tela.length();
    let bico = a + dir * 46.;
    crate::gizmo::arrow(&ui.painter_at(rect), a + dir * RAIO_MARCA, bico, cor);
    let id = egui::Id::new("rep_seta");
    let resp = ui.interact(Rect::from_center_size(bico, egui::vec2(26., 26.)), id, Sense::drag());
    if !resp.dragged() {
        return false;
    }
    let novo = espacamento_arrastado(atual, resp.drag_delta().dot(dir), px_mm);
    if (novo - atual).abs() < 1e-9 || novo <= 0. {
        return false;
    }
    *spacing = fmt_num(novo);
    ui.painter_at(rect).text(bico + egui::vec2(12., -12.), egui::Align2::LEFT_BOTTOM, format!("{} mm", fmt(novo)), egui::FontId::proportional(12.), cor);
    true
}

pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    if app.settings.bench != crate::settings::Bench::Montagem {
        return;
    }
    let Some(st) = app.asm.as_mut() else { return };
    let Some(mut e) = st.repeticao.take() else { return };
    let mudou = quadro(st, app.result.as_ref(), &mut e, ui, rect, cam);
    st.repeticao = Some(e);
    if mudou {
        st.fantasma_novo = true;
    }
}

/// `FENKRA_DEMO_ASM=repeticao`: placa com direção clicada, uma cópia pulada.
pub fn demo(st: &mut AsmState) {
    let ids: Vec<Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| n.id).collect();
    let Some(&src) = ids.get(1) else { return };
    st.asm.node_mut(ids[0]).map(|n| n.fixed = true);
    st.asm.node_mut(src).map(|n| n.placement.origin = [0., 80., 0.]);
    st.sel = Some(src);
    st.dirty = true;
    st.refresh();
    abrir(st);
    let Some(e) = st.repeticao.as_mut() else { return };
    e.r.ref1 = Some(MateRef { comp: ids[0], geom: Geom::Axis { p: [0.; 3], d: [1., 0., 0.] }, pick: None, viva: None });
    e.nomes[0] = st.asm.name_of(ids[0]);
    e.r.kind = PatternKind::Linear { dir: [1., 0., 0.], count: "4".into(), spacing: "100".into(), dir2: None, count2: "2".into(), spacing2: "0".into() };
    e.r.pular = vec![(2, 0)];
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};
    use fenkra_montagem::Placement;

    fn montagem() -> (AsmState, Id, Id) {
        let mut a = Assembly::new("t");
        let g = a.add_component("Guia", NodeKind::Part { path: "x.fkp".into() }, Placement::default(), None);
        let s = a.add_component("S", NodeKind::Part { path: "x.fkp".into() }, Placement { origin: [0., 50., 0.], ..Default::default() }, None);
        let mut st = AsmState::new(a, None);
        st.sel = Some(s);
        (st, g, s)
    }

    fn com_direcao(st: &mut AsmState, g: Id) {
        let e = st.repeticao.as_mut().unwrap();
        e.r.ref1 = Some(MateRef { comp: g, geom: Geom::Axis { p: [0.; 3], d: [1., 0., 0.] }, pick: None, viva: None });
        if let PatternKind::Linear { spacing, .. } = &mut e.r.kind {
            *spacing = "40".into();
        }
    }

    #[test]
    fn nasce_sem_direcao_e_nada_entra_antes_de_aceitar() {
        let (mut st, g, _) = montagem();
        abrir(&mut st);
        assert!(!st.repeticao.as_ref().unwrap().pronto(), "sem clique não há direção");
        assert_eq!(aceitar(&mut st), None);
        assert!(st.asm.patterns.is_empty());
        com_direcao(&mut st, g);
        assert!(st.repeticao.as_ref().unwrap().pronto());
        assert_eq!(st.asm.nodes.len(), 2, "o rascunho não cria nó");
        cancelar(&mut st);
        assert!(st.asm.patterns.is_empty() && st.undo.is_empty());
    }

    #[test]
    fn aceitar_grava_com_um_desfazer_e_guarda_a_direcao_da_referencia() {
        let (mut st, g, _) = montagem();
        abrir(&mut st);
        com_direcao(&mut st, g);
        let id = aceitar(&mut st).expect("aceito");
        assert_eq!(st.undo.len(), 1);
        let p = st.asm.patterns.iter().find(|p| p.id == id).unwrap();
        assert_eq!(p.instances.len(), 2);
        assert!(matches!(p.kind, PatternKind::Linear { dir: [1., 0., 0.], .. }));
        assert_eq!(st.asm.node(p.instances[1]).unwrap().placement.origin, [80., 50., 0.]);
    }

    #[test]
    fn editar_e_cancelar_devolve_as_mesmas_copias_e_aceitar_mantem_os_ids() {
        let (mut st, g, _) = montagem();
        abrir(&mut st);
        com_direcao(&mut st, g);
        let id = aceitar(&mut st).unwrap();
        let copias = st.asm.patterns[0].instances.clone();
        editar(&mut st, id);
        assert!(st.asm.patterns[0].instances.is_empty(), "as cópias saem da vista enquanto o rascunho manda");
        cancelar(&mut st);
        assert_eq!(st.asm.patterns[0].instances, copias);
        editar(&mut st, id);
        if let PatternKind::Linear { spacing, .. } = &mut st.repeticao.as_mut().unwrap().r.kind {
            *spacing = "60".into();
        }
        aceitar(&mut st).unwrap();
        assert_eq!(st.asm.patterns[0].instances, copias, "restrições nas cópias continuam valendo");
        assert_eq!(st.asm.node(copias[0]).unwrap().placement.origin, [60., 50., 0.]);
    }

    #[test]
    fn clicar_nas_marcas_pula_e_a_seta_arrasta_o_espacamento() {
        let (mut st, g, _) = montagem();
        abrir(&mut st);
        com_direcao(&mut st, g);
        let tela = Rect::from_min_size(pos2(0., 0.), vec2(1000., 700.));
        let cam = Camera { dist: 400., ..Default::default() };
        let ctx = egui::Context::default();
        let passo = |st: &mut AsmState, evs: Vec<egui::Event>| {
            let raw = egui::RawInput { screen_rect: Some(tela), events: evs, ..Default::default() };
            let mut mudou = false;
            let mut saida = ctx.run_ui(raw, |ui| {
                let mut e = st.repeticao.take().unwrap();
                mudou = quadro(st, None, &mut e, ui, tela, &cam);
                st.repeticao = Some(e);
            });
            saida.textures_delta.clear();
            mudou
        };
        let botao = |p: egui::Pos2, pressed| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        passo(&mut st, vec![]);
        // marca da segunda cópia: centro é a origem da peça, sem vista neste teste
        let e = st.repeticao.as_ref().unwrap();
        let passos = st.asm.passos(&e.r, &st.asm.vars());
        let centro = st.asm.node(e.r.source).unwrap().placement.origin;
        let marca = cam.project(tela, v3(centro_no_passo(&passos[1], centro))).unwrap();
        passo(&mut st, vec![egui::Event::PointerMoved(marca)]);
        passo(&mut st, vec![botao(marca, true)]);
        let mudou = passo(&mut st, vec![botao(marca, false)]);
        assert!(mudou, "a marca respondeu ao clique");
        assert_eq!(st.repeticao.as_ref().unwrap().r.pular, vec![(2, 0)]);
        // seta: na primeira cópia, 46 pontos para o lado da direção
        let base = centro_no_passo(&passos[0], centro);
        let (a, b) = (cam.project(tela, v3(base)).unwrap(), cam.project(tela, v3([base[0] + 10., base[1], base[2]])).unwrap());
        let dir = (b - a) / (b - a).length();
        let bico = a + dir * 46.;
        passo(&mut st, vec![egui::Event::PointerMoved(bico)]);
        passo(&mut st, vec![botao(bico, true)]);
        passo(&mut st, vec![egui::Event::PointerMoved(bico + dir * 30.)]);
        passo(&mut st, vec![egui::Event::PointerMoved(bico + dir * 60.)]);
        passo(&mut st, vec![botao(bico + dir * 60., false)]);
        let PatternKind::Linear { spacing, .. } = &st.repeticao.as_ref().unwrap().r.kind else { panic!() };
        assert!(spacing.parse::<f64>().unwrap() > 40., "arrastar para fora aumenta: {spacing}");
    }

    #[test]
    fn espacamento_da_seta_segue_a_escala_da_tela() {
        assert_eq!(espacamento_arrastado(20., 30., 3.), 30.);
        assert_eq!(espacamento_arrastado(20., -30., 3.), 10.);
    }

    #[test]
    fn varios_componentes_somam_e_tiram_com_o_clique() {
        let mut e = Estado::novo(&[]);
        assert_eq!(e.alvo, Alvo::Componentes);
        e.alternar(Id(5));
        e.alternar(Id(7));
        assert_eq!((e.r.source, e.r.extras.clone()), (Id(5), vec![Id(7)]));
        e.alternar(Id(5));
        assert_eq!((e.r.source, e.r.extras.clone()), (Id(7), vec![]));
    }
}
