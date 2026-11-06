// SPDX-License-Identifier: LGPL-2.1-or-later
//! Painel e prévia do comando de restrição e junta (B244). A lógica mora em
//! `fenkra_montagem::proposta`; aqui ficam o clique, o painel, as teclas e o fantasma. Nada grava na
//! montagem antes de aceitar: o comando resolve numa cópia, e a peça aparece na posição final como
//! fantasma, com uma seta na normal de cada elemento.

use crate::app::App;
use crate::asm_ui::{AsmState, MateCmd};
use crate::i18n::tr;
use crate::picking::Pick;
use eframe::egui::{self, Color32, RichText};
use fenkra_montagem::kin::{self, JointKind};
use fenkra_montagem::proposta::{animar, Classe, Comando, Elemento, ModoOrigem, Movido, OpcoesAnimar, Previa, Tipo};
use fenkra_montagem::*;

/// Comando aberto: a proposta, o que foi clicado e o último resultado.
pub struct Pendente {
    pub c: Comando,
    pub picks: [Vec<Pick>; 2],
    pub textos: [Vec<String>; 2],
    pub resultado: Option<Result<Previa, String>>,
    /// proposta que gerou `resultado`
    resolvida: Option<Comando>,
    /// o último clique não serviu
    pub aviso: Option<String>,
    /// quadros do Animar e a hora em que começou
    animacao: Option<(Vec<Vec<Movido>>, f64)>,
    opcoes_animar: OpcoesAnimar,
    /// pontos de encaixe da face sob o cursor (B99)
    pub encaixe: crate::asm_encaixe::Encaixe,
    /// deslocamento em X, Y e Z, com fórmula
    formulas: [String; 3],
    /// restrição do conflito apontada no painel: ela acende no 3D
    conflito_sob_mouse: Option<fenkra_core::Id>,
}

/// Quadros por segundo do Animar.
const QUADROS_POR_SEGUNDO: f64 = 24.;

impl Pendente {
    fn novo(tipo: Tipo) -> Self {
        Pendente { c: Comando::novo(tipo), picks: Default::default(), textos: Default::default(), resultado: None, resolvida: None, aviso: None, animacao: None, opcoes_animar: Default::default(), encaixe: Default::default(), formulas: ["0".into(), "0".into(), "0".into()], conflito_sob_mouse: None }
    }

    /// Peças fora do lugar agora: as da prévia ou as do quadro do Animar.
    fn movidos_em(&self, t: f64) -> &[Movido] {
        match &self.animacao {
            Some((q, t0)) if !q.is_empty() => &q[((t - t0).max(0.) * QUADROS_POR_SEGUNDO) as usize % q.len()],
            _ => self.previa().map_or(&[], |p| p.movidos.as_slice()),
        }
    }

    /// Guarda o que entrou na caixa `k`: a segunda referência da origem soma, a primeira troca.
    fn anotar(&mut self, k: usize, texto: String, pick: Option<Pick>) {
        if self.c.segundos[k].is_none() {
            self.textos[k].clear();
            self.picks[k].clear();
        }
        self.textos[k].push(texto);
        self.picks[k].extend(pick);
    }

    pub fn previa(&self) -> Option<&Previa> {
        self.resultado.as_ref().and_then(|r| r.as_ref().ok())
    }
}

/// Junta pelo tipo que a geometria pedir.
const AUTOMATICA: u8 = 255;

/// Modo do comando pedido pela barra.
pub fn tipo_da_restricao(m: MateCmd) -> Tipo {
    match m {
        MateCmd::Auto => Tipo::Automatica,
        MateCmd::AlignOnce => Tipo::Alinhar,
        m => m.kind().map(Tipo::Restricao).unwrap_or(Tipo::Automatica),
    }
}

pub fn tipo_da_junta(k: u8) -> Tipo {
    if k == AUTOMATICA {
        return Tipo::JuntaAutomatica;
    }
    Tipo::Junta(match k {
        0 => JointKind::Revolute,
        1 => JointKind::Prismatic,
        2 => JointKind::Cylindrical,
        3 => JointKind::Screw { pitch: "2".into() },
        4 => JointKind::Spherical,
        5 => JointKind::Planar,
        6 => JointKind::Rigid,
        7 => JointKind::PointCurve { curve: vec![] },
        _ => JointKind::Cam { profile: vec![], radius: "0".into() },
    })
}

/// Abre o comando. Os elementos já escolhidos entram nas caixas, sem repetir a seleção.
pub fn abrir(app: &mut App, tipo: Tipo) {
    let picks = std::mem::take(&mut app.picks);
    let Some(st) = app.asm.as_mut() else { return };
    st.sel = None;
    st.props = false;
    st.cmd = Some(Pendente::novo(tipo));
    if let Some(item) = st.origem_sel.take() {
        receber_origem(st, item);
    }
    for p in picks.into_iter().take(2) {
        receber(st, p);
    }
    atualizar(st);
}

/// Item da origem da montagem como elemento do comando (clique na árvore).
pub fn receber_origem(st: &mut AsmState, item: crate::asm_selecao::ItemOrigem) {
    let Some(pen) = st.cmd.as_mut() else { return };
    let k = pen.c.receber(item.elemento());
    pen.anotar(k, format!("{} · {}", item.nome(), tr("Origem")), None);
    pen.aviso = None;
    atualizar(st);
}

/// Texto do elemento, como a pessoa o reconhece: "Eixo de Suporte".
fn descrever(e: &Elemento, nome: &str) -> String {
    let t = match (e.classe, e.r.geom) {
        (Classe::Vertice, _) => "Vértice",
        (Classe::ArestaReta, _) => "Aresta reta",
        (Classe::ArestaCurva { circular: true }, _) => "Aresta circular",
        (Classe::ArestaCurva { .. }, _) => "Aresta curva",
        (Classe::Face, Geom::Axis { .. }) => "Eixo",
        (Classe::Face, _) => "Face plana",
    };
    format!("{} · {nome}", tr(t))
}

/// Elemento a partir do clique na viewport.
fn elemento_de(st: &mut AsmState, p: Pick) -> Result<Elemento, String> {
    let r = st.mate_ref(p)?;
    let mut e = Elemento::novo(r, match p {
        Pick::Vertex { .. } => Classe::Vertice,
        _ => Classe::Face,
    });
    if let Pick::Edge { body, edge, mid } = p {
        let pl = st.asm.node(r.comp).map(|n| n.placement).unwrap_or_default();
        let mundo: Vec<[f64; 3]> = st.view.as_ref().map(|v| crate::picking::edge_polyline(v, body, edge)).unwrap_or_default();
        e.curva = mundo.iter().map(|q| pl.to_local(*q)).collect();
        let curva = fenkra_solidos::geo3d::Curve3::poly(mundo.clone(), false).ok();
        e.classe = match &curva {
            Some(c) if c.is_straight() => Classe::ArestaReta,
            Some(c) => {
                let circulo = fenkra_solidos::geo3d::fit_circle(c).ok();
                e.raio = circulo.as_ref().map(|c| c.2);
                let circular = circulo.is_some_and(|(ctr, _, raio)| mundo.iter().all(|q| (norm(sub(*q, ctr)) - raio).abs() < raio.max(1.) * 1e-3));
                Classe::ArestaCurva { circular }
            }
            None => Classe::ArestaReta,
        };
        if e.curva.len() >= 3 {
            e.perfil = Some(perfil_do_came(st, r.comp, &e.curva, pl.to_local(mid.map(|v| v as f64))));
        }
    }
    Ok(e)
}

/// Perfil do came: o contorno da seção da peça pelo plano da aresta (vários arcos viram um perfil
/// só). Sem seção, fica a própria aresta.
fn perfil_do_came(st: &mut AsmState, comp: fenkra_core::Id, aresta: &[[f64; 3]], meio: [f64; 3]) -> Vec<[f64; 3]> {
    // normal do plano da aresta (Newell; aresta reta: cai no eixo z da peça)
    let mut nn = [0.; 3];
    for i in 0..aresta.len() {
        let (p, q) = (aresta[i], aresta[(i + 1) % aresta.len()]);
        nn[0] += (p[1] - q[1]) * (p[2] + q[2]);
        nn[1] += (p[2] - q[2]) * (p[0] + q[0]);
        nn[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    let nn = if norm(nn) > 1e-9 { unit(nn) } else { [0., 0., 1.] };
    let dir = st.dir();
    let forma = match st.asm.node(comp).map(|n| n.kind.clone()) {
        Some(NodeKind::Part { path }) => st.cache.get(&load::full_path(dir.as_deref(), &path), true).ok().and_then(|p| p.shape.clone()),
        _ => None,
    };
    let lacos = forma.and_then(|s| s.section_loops(meio, nn, 1e4).ok()).unwrap_or_default();
    let dist = |l: &Vec<[f64; 3]>| l.iter().map(|p| norm(sub(*p, meio))).fold(f64::MAX, f64::min);
    lacos.into_iter().min_by(|x, y| dist(x).total_cmp(&dist(y))).unwrap_or_else(|| aresta.to_vec())
}

fn receber(st: &mut AsmState, p: Pick) {
    // ponto de encaixe aceso ou travado vale no lugar da face inteira
    let ponto = st.cmd.as_ref().and_then(|c| c.encaixe.escolhido()).and_then(|(b, c)| crate::asm_encaixe::elemento(st, b, &c));
    let achado = ponto.map_or_else(|| elemento_de(st, p), Ok);
    if let Some(c) = st.cmd.as_mut() {
        c.encaixe.travado = None;
    }
    let nome = achado.as_ref().map(|e| st.asm.name_of(e.r.comp)).unwrap_or_default();
    let Some(pen) = st.cmd.as_mut() else { return };
    match achado {
        Ok(e) => {
            let texto = descrever(&e, &nome);
            let k = pen.c.receber(e);
            pen.anotar(k, texto, Some(p));
            pen.aviso = None;
        }
        Err(e) => pen.aviso = Some(e),
    }
}

/// Refaz a prévia se a proposta mudou.
pub fn atualizar(st: &mut AsmState) {
    let Some(pen) = st.cmd.as_mut() else { return };
    if pen.resolvida.as_ref() == Some(&pen.c) {
        return;
    }
    pen.resultado = pen.c.completo().then(|| pen.c.resolver(&st.asm));
    pen.resolvida = Some(pen.c.clone());
    // o que estava animando era a proposta de antes
    pen.animacao = None;
    st.fantasma_novo = true;
}

/// Cliques na viewport com o comando aberto: cada um vai para a caixa ativa.
pub fn apos_viewport(app: &mut App) -> bool {
    if !app.asm.as_ref().is_some_and(|s| s.cmd.is_some()) {
        return false;
    }
    let picks = std::mem::take(&mut app.picks);
    if let Some(st) = app.asm.as_mut() {
        for p in picks {
            receber(st, p);
        }
        atualizar(st);
    }
    true
}

/// Grava a proposta na montagem. `continuar` deixa o comando aberto para o próximo par.
pub fn aceitar(app: &mut App, continuar: bool) {
    let Some(st) = app.asm.as_mut() else { return };
    let Some(pen) = st.cmd.as_ref() else { return };
    // resolve de novo: a montagem pode ter mudado (desfazer) depois da última prévia
    let previa = match pen.c.resolver(&st.asm) {
        Ok(p) if !p.falhou() => p,
        Ok(_) => return app.toast(tr("Sem solução: a montagem não mudou. Ajuste a proposta ou cancele."), true),
        Err(e) => return app.toast(tr(&e).to_string(), true),
    };
    st.push_undo();
    st.asm = previa.copia;
    st.solve();
    let msg = match previa.id {
        None => tr("Alinhado (sem restrição guardada)").to_string(),
        Some(id) if st.asm.joints.iter().any(|j| j.id == id) => {
            let (sem, com) = kin::analyze(&st.asm);
            format!("{} {} · {sem} {} ({com} {})", st.asm.name_of(id), tr("criada"), tr("GDL no mecanismo"), tr("com os comandos"))
        }
        Some(id) => format!("{} {} · {} {}", st.asm.name_of(id), tr("criada"), st.report.dof, tr("GDL livres")),
    };
    if continuar {
        if let Some(pen) = st.cmd.as_mut() {
            let tipo = pen.c.tipo.clone();
            *pen = Pendente::novo(tipo);
        }
    } else {
        st.cmd = None;
    }
    st.fantasma_novo = true;
    atualizar(st);
    app.toast(msg, false);
}

/// Fecha o comando. Como nada foi gravado, não há o que desfazer.
pub fn cancelar(app: &mut App) {
    app.picks.clear();
    if let Some(st) = app.asm.as_mut() {
        st.cmd = None;
        st.fantasma_novo = true;
    }
}

fn inverter(st: &mut AsmState) {
    if let Some(pen) = st.cmd.as_mut() {
        let previa = pen.resultado.as_ref().and_then(|r| r.as_ref().ok()).cloned();
        pen.c.inverter(previa.as_ref());
    }
    atualizar(st);
}

/// Enter aceita e continua, Tab inverte e Esc cancela. Roda antes dos painéis, para o Tab não virar
/// troca de foco.
pub fn teclas(app: &mut App, ctx: &egui::Context) {
    if !app.asm.as_ref().is_some_and(|s| s.cmd.is_some()) || ctx.egui_wants_keyboard_input() {
        return;
    }
    let (enter, tab, esc) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Tab), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)));
    if esc {
        cancelar(app);
    } else if enter {
        if app.asm.as_ref().and_then(|s| s.cmd.as_ref()).is_some_and(|p| p.c.completo()) {
            aceitar(app, true);
        }
    } else if tab {
        if let Some(st) = app.asm.as_mut() {
            inverter(st);
        }
    }
}

/// Cor de cada caixa, igual à da seta no 3D.
fn cor(v: &egui::Visuals, k: usize) -> Color32 {
    if k == 0 {
        crate::theme::accent(v)
    } else {
        crate::theme::st_ok()
    }
}

/// Aviso fixo da etapa, para a instrução não sumir como um aviso temporário.
fn etapa_texto(c: &Comando) -> &'static str {
    let trajetoria = c.rotulos()[0] == "Trajetória";
    match c.etapa() {
        2 => "Etapa 3 de 3: confira o fantasma; Enter aceita, Tab inverte, Esc cancela",
        _ if c.pede_segundo(c.ativa) => "Falta a segunda referência da origem nesta caixa",
        _ if trajetoria => "Clique na trajetória e no seguidor, em qualquer ordem",
        _ if c.ativa == 0 => "Etapa 1 de 3: clique no primeiro elemento",
        _ => "Etapa 2 de 3: clique no segundo elemento",
    }
}

const RESTRICOES: [(MateCmd, &str, &str); 11] = [
    (MateCmd::Auto, "rest_auto", "Escolhe o tipo mais lógico para o par."),
    (MateCmd::Contact, "rest_contato", "Faces encostadas, de frente uma para a outra, sem folga."),
    (MateCmd::Coincident, "rest_coincidencia", "Alinha eixos, junta pontos ou deixa faces coplanares (direta ou invertida)."),
    (MateCmd::Distance, "rest_distancia", "Separação entre faces, eixos ou pontos (positiva, negativa ou nula; aceita fórmula)."),
    (MateCmd::Angle, "rest_angulo", "Inclinação entre faces ou linhas."),
    (MateCmd::Parallel, "rest_paralelo", "Faces ou eixos paralelos (rígido)."),
    (MateCmd::Perpendicular, "rest_perpendicular", "Faces ou eixos a 90° (rígido)."),
    (MateCmd::Gear, "rest_engrenagem", "Dois eixos: o giro de um vira o giro do outro pela razão (externas: −z1/z2)."),
    (MateCmd::Pulley, "rest_polia", "Dois eixos girando no mesmo sentido pela razão dos diâmetros."),
    (MateCmd::RackPinion, "rest_cremalheira", "Eixo do pinhão e aresta da cremalheira: avanço = mm por volta × voltas."),
    (MateCmd::AlignOnce, "rest_alinhar", "Posiciona pelos dois elementos escolhidos sem guardar restrição (sem histórico)."),
];

const JUNTAS: [(&str, &str); 9] = [
    ("junta_revolucao", "Gira em torno de um eixo."),
    ("junta_prismatica", "Desliza ao longo de um trilho."),
    ("junta_cilindrica", "Gira e desliza no mesmo eixo."),
    ("junta_parafuso", "Rotação e avanço ligados pelo passo."),
    ("junta_esferica", "Três rotações livres no ponto."),
    ("junta_planar", "Duas translações e uma rotação sobre o plano."),
    ("junta_rigida", "Sem graus de liberdade entre as duas peças."),
    ("junta_ponto_curva", "O ponto da segunda peça segue a trajetória."),
    ("junta_came", "O seguidor fica encostado no perfil do came."),
];

fn botao_modo(ui: &mut egui::Ui, icone: &str, nome: &str, dica: &str, ligado: bool) -> bool {
    let img = egui::Image::new(crate::icons::get(icone)).fit_to_exact_size(egui::vec2(18., 18.));
    ui.add(egui::Button::image_and_text(img, tr(nome)).selected(ligado)).on_hover_text(tr(dica)).clicked()
}

/// Painel do comando: suporte, modo, parâmetros e avançado.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    let mut acao = None;
    let Some(st) = app.asm.as_mut() else { return };
    let vars = st.asm.vars();
    let nomes_conflito: Vec<(fenkra_core::Id, String)> = st.cmd.as_ref().and_then(|p| p.previa()).map(|p| p.conflito.envolvidas.iter().map(|f| (*f, p.copia.name_of(*f))).collect()).unwrap_or_default();
    let Some(pen) = st.cmd.as_mut() else { return };
    let visuais = ui.visuals().clone();
    let acc = crate::theme::accent(&visuais);
    let completo = pen.c.completo();
    let pode = completo && pen.previa().is_some_and(|p| !p.falhou());
    ui.horizontal(|ui| {
        if ui.add_enabled(pode, egui::Button::new(RichText::new(tr("✔ Aceitar")).strong()).min_size(egui::vec2(96., 28.))).on_hover_text(tr("Enter: grava e continua no comando")).clicked() {
            acao = Some(0);
        }
        if ui.add(egui::Button::new(tr("✔ Concluir")).min_size(egui::vec2(90., 28.))).on_hover_text(tr("Grava o que estiver pronto e fecha o comando")).clicked() {
            acao = Some(if pode { 1 } else { 2 });
        }
        if ui.add(egui::Button::new(tr("✖ Cancelar")).min_size(egui::vec2(90., 28.))).on_hover_text(tr("Esc: fecha sem mudar a montagem")).clicked() {
            acao = Some(2);
        }
    });
    ui.add_space(4.);
    egui::Frame::NONE.fill(acc.gamma_multiply(0.15)).inner_margin(6.).corner_radius(4.).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr(pen.c.tipo.label())).strong());
        ui.label(tr(etapa_texto(&pen.c)));
    });
    // ---- resultado da prévia: no topo, porque é a primeira coisa a ler quando não fecha
    match &pen.resultado {
        Some(Err(e)) => {
            ui.label(RichText::new(tr(e)).color(crate::theme::st_err()));
        }
        Some(Ok(p)) if p.falhou() => {
            quadro_de_conflito(ui, p, &nomes_conflito, &mut pen.conflito_sob_mouse);
            ui.label(RichText::new(format!("{} {}", p.report.dof, tr("GDL restariam com a proposta"))).small());
        }
        Some(Ok(p)) => {
            ui.label(RichText::new(format!("{} {} ({} {})", p.report.dof, tr("GDL livres depois de aceitar"), p.gdl_antes, tr("agora"))).color(crate::theme::st_ok()));
            if p.redundante() {
                ui.label(RichText::new(tr("Não tira grau de liberdade: repete o que já está restrito.")).small().color(crate::theme::st_warn()));
            }
        }
        None => {}
    }
    // ---- suporte: duas caixas, com a cor das setas do 3D
    crate::editor::section(ui, "Suporte");
    for k in 0..2 {
        let c = cor(&visuais, k);
        let ativa = pen.c.ativa == k;
        let cheia = pen.c.caixas[k].is_some();
        let quadro = egui::Frame::NONE.fill(c.gamma_multiply(if ativa { 0.22 } else { 0.08 })).stroke(egui::Stroke::new(if ativa { 2. } else { 1. }, c)).inner_margin(6.).corner_radius(4.);
        let r = quadro
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr(pen.c.rotulos()[k])).strong().color(crate::theme::readable(c, &visuais)));
                    let t = if cheia { RichText::new(pen.textos[k].join(" + ")) } else { RichText::new(tr("vazio: clique no 3D")).weak() };
                    ui.label(t);
                    if cheia {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✖").on_hover_text(tr("Esvazia esta caixa e volta a ela")).clicked() {
                                pen.c.limpar_caixa(k);
                                pen.picks[k].clear();
                                pen.textos[k].clear();
                            }
                        });
                    }
                });
            })
            .response
            .interact(egui::Sense::click())
            .on_hover_text(tr("Clique para trocar este elemento: o próximo clique no 3D vem para cá"));
        if r.clicked() {
            pen.c.ativa = k;
        }
        // origem da junta nesta peça
        if pen.c.tipo.e_junta() && pen.c.rotulos()[0] != "Trajetória" {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Origem")).small());
                for (m, nome, dica) in [
                    (ModoOrigem::EmPonto, "No elemento", "A origem da junta fica no elemento clicado."),
                    (ModoOrigem::EntreFaces, "Entre duas faces", "Clique em duas faces paralelas: a origem fica no meio."),
                    (ModoOrigem::IntersecaoArestas, "Interseção de arestas", "Clique em duas arestas retas: a origem fica no cruzamento."),
                ] {
                    if ui.selectable_label(pen.c.modos[k] == m, RichText::new(tr(nome)).small()).on_hover_text(tr(dica)).clicked() {
                        pen.c.trocar_modo_origem(k, m);
                        pen.textos[k].truncate(1);
                        pen.picks[k].truncate(1);
                    }
                }
            });
        }
        ui.add_space(2.);
    }
    if pen.c.rotulos()[0] == "Trajetória" && completo && ui.small_button(tr("⇄ Trocar trajetória e seguidor")).on_hover_text(tr("Para um came redondo clicado na ordem contrária")).clicked() {
        pen.c.trocar_caixas();
        pen.textos.swap(0, 1);
        pen.picks.swap(0, 1);
    }
    if let Some(a) = &pen.aviso {
        ui.label(RichText::new(tr(a)).color(crate::theme::st_warn()).small());
    }
    // ---- modo: trocar atualiza a prévia, sem repetir a seleção
    crate::editor::section(ui, "Modo");
    let junta = pen.c.tipo.e_junta();
    let mut novo = None;
    ui.horizontal_wrapped(|ui| {
        if junta {
            if botao_modo(ui, "rest_auto", "Automática", "Reconhece a junta pela geometria: dois furos giram, duas arestas retas deslizam.", pen.c.tipo == Tipo::JuntaAutomatica) {
                novo = Some(Tipo::JuntaAutomatica);
            }
            for (k, (icone, dica)) in JUNTAS.iter().enumerate() {
                let t = tipo_da_junta(k as u8);
                if botao_modo(ui, icone, t.label(), dica, t.label() == pen.c.tipo.label()) {
                    novo = Some(t);
                }
            }
        } else {
            for (m, icone, dica) in RESTRICOES {
                let t = tipo_da_restricao(m);
                if botao_modo(ui, icone, t.label(), dica, t.label() == pen.c.tipo.label()) {
                    novo = Some(t);
                }
            }
        }
    });
    if let Some(t) = novo {
        pen.c.trocar_tipo(t);
    }
    // ---- parâmetros
    crate::editor::section(ui, "Parâmetros");
    let previa = pen.resultado.as_ref().and_then(|r| r.as_ref().ok());
    let (aceita_alin, aceita_lado) = previa.map(|p| (p.aceita_alinhamento, p.aceita_lado)).unwrap_or((false, false));
    let medida = previa.and_then(|p| p.valor.clone());
    let campo = match &pen.c.tipo {
        Tipo::Restricao(MateKind::Distance(_)) => Some(("Valor", "mm", "Começa na medida de agora; aceita fórmula com os parâmetros da montagem.")),
        Tipo::Restricao(MateKind::Angle(_)) => Some(("Valor", "°", "Começa no ângulo de agora; aceita fórmula com os parâmetros da montagem.")),
        Tipo::Restricao(MateKind::Gear { .. }) => Some(("Razão", "", "giro da 2ª ÷ giro da 1ª")),
        Tipo::Restricao(MateKind::RackPinion { .. }) => Some(("mm por volta", "mm", "π × diâmetro primitivo do pinhão")),
        _ => None,
    };
    if let Some((rotulo, unidade, dica)) = campo {
        let mut txt = pen.c.valor.clone().or(medida).unwrap_or_default();
        let antes = txt.clone();
        crate::editor::expr(ui, rotulo, &mut txt, &vars, unidade);
        ui.label(RichText::new(tr(dica)).weak().small());
        if txt != antes {
            pen.c.valor = Some(txt);
        }
    }
    if let Tipo::Junta(JointKind::Screw { pitch }) = &mut pen.c.tipo {
        crate::editor::expr(ui, "Passo (mm/volta)", pitch, &vars, "mm");
        ui.label(RichText::new(tr("Avanço a cada volta completa.")).weak().small());
    }
    if let Tipo::Junta(JointKind::Cam { radius, .. }) = &mut pen.c.tipo {
        crate::editor::expr(ui, "Raio do rolete", radius, &vars, "mm");
        ui.label(RichText::new(tr("Vem da aresta circular do seguidor; zero é seguidor de ponta.")).weak().small());
    }
    if junta {
        let reconhecida = pen.previa().and_then(|p| p.id.and_then(|id| p.copia.joints.iter().find(|j| j.id == id)));
        let graus = reconhecida.map_or(&[][..], |j| j.kind.graus());
        let medidas: Vec<(kin::Grau, f64)> = pen.previa().zip(reconhecida).map_or(vec![], |(p, j)| {
            let lugar = |id| p.copia.lugar(id).unwrap_or_default();
            graus.iter().map(|g| (*g, kin::medir_grau(j, *g, &lugar(j.a.comp), &lugar(j.b.comp)))).collect()
        });
        if let (Tipo::JuntaAutomatica, Some(j)) = (&pen.c.tipo, reconhecida) {
            ui.label(RichText::new(format!("{} {}", tr("Reconhecida:"), tr(j.kind.label()))).strong());
        }
        let al = &mut pen.c.alinhamento;
        ui.horizontal(|ui| {
            ui.label(tr("Ângulo inicial"));
            ui.add(crate::ui_ext::dv(&mut al.angulo).speed(1.).suffix("°").dica("Gira a peça em volta do eixo da junta antes de aceitar"));
        });
        for (k, l) in ["Deslocamento X", "Deslocamento Y", "Deslocamento Z"].into_iter().enumerate() {
            crate::editor::expr(ui, l, &mut pen.formulas[k], &vars, "mm");
        }
        let (desloc, ruim) = fenkra_montagem::encaixe::avaliar_desloc(&pen.formulas, &vars);
        al.desloc = desloc;
        if ruim.is_some() {
            ui.label(RichText::new(tr("Fórmula inválida: conta como zero")).small().color(crate::theme::st_err()));
        }
        ui.horizontal(|ui| {
            if ui.selectable_label(al.inverter, tr("Inverter")).on_hover_text(tr("Tab: a peça vira, e o eixo dela passa a apontar ao contrário")).clicked() {
                al.inverter = !al.inverter;
            }
            if ui.button(tr("Girar 90°")).on_hover_text(tr("Gira a peça de 90 em 90 graus em volta do eixo principal.")).clicked() {
                al.angulo = (al.angulo + 90.) % 360.;
            }
        });
        ui.label(RichText::new(tr("Na vista: passe o mouse numa face para ver os pontos de encaixe; a tecla L trava o ponto.")).weak().small());
        // ---- animar: ir e voltar no 3D, sem a janela de simulação
        ui.add_space(4.);
        ui.horizontal(|ui| {
            let ligado = pen.animacao.is_some();
            let texto = if ligado { tr("⏹ Parar") } else { tr("▶ Animar") };
            if ui.add_enabled(pode, egui::Button::new(RichText::new(texto).strong())).on_hover_text(tr("A peça vai e volta no 3D, antes de aceitar")).clicked() {
                acao = Some(3);
            }
            if ui.checkbox(&mut pen.opcoes_animar.limites, tr("incluir os limites")).on_hover_text(tr("Vai até os limites da junta, em vez da faixa padrão")).changed() {
                pen.animacao = None;
            }
        });
        // faixa de movimento: o solucionador a impõe, e a prévia já respeita
        egui::CollapsingHeader::new(tr("Limites")).id_salt("asm_cmd_limites").show(ui, |ui| {
            let medida = |g| medidas.iter().find(|m| m.0 == g).map(|m| m.1);
            crate::asm_limites::painel(ui, "asm_cmd_limites_graus", graus, &mut pen.c.limites, medida, |_| false);
        });
    }
    if aceita_alin || aceita_lado {
        ui.horizontal(|ui| {
            if aceita_alin && ui.selectable_label(pen.c.inv_alinhamento, tr("Inverter alinhamento")).on_hover_text(tr(crate::asm_ui::DICA_ALINHAMENTO)).clicked() {
                pen.c.inv_alinhamento = !pen.c.inv_alinhamento;
            }
            if aceita_lado && ui.selectable_label(pen.c.inv_lado, tr("Inverter lado")).on_hover_text(tr(crate::asm_ui::DICA_LADO)).clicked() {
                pen.c.inv_lado = !pen.c.inv_lado;
            }
        });
    }
    // ---- avançado
    egui::CollapsingHeader::new(tr("Avançado")).id_salt("asm_cmd_avancado").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Nome"));
            ui.text_edit_singleline(&mut pen.c.nome).on_hover_text(tr("Vazio: o nome é gerado pelo tipo"));
        });
        crate::editor::check(ui, &mut pen.c.suprimida, "suprimida").on_hover_text(tr("Nasce guardada, sem efeito sobre as peças"));
    });
    atualizar(st);
    match acao {
        Some(0) => aceitar(app, true),
        Some(1) => aceitar(app, false),
        Some(2) => cancelar(app),
        Some(3) => {
            let agora = ui.input(|i| i.time);
            alternar_animacao(app, agora);
        }
        _ => {}
    }
}

/// Por que a proposta não fecha, com cada restrição do conflito em vermelho.
fn quadro_de_conflito(ui: &mut egui::Ui, p: &Previa, nomes: &[(fenkra_core::Id, String)], sob_mouse: &mut Option<fenkra_core::Id>) {
    let erro = crate::theme::st_err();
    ui.label(RichText::new(tr("✖ Sem solução; a montagem não muda.")).color(erro));
    *sob_mouse = None;
    if p.conflito.sozinho {
        ui.label(RichText::new(tr("Não fecha nem sozinha: confira as peças fixas e o valor pedido.")).small().color(erro));
    } else if nomes.is_empty() {
        ui.label(RichText::new(tr("O solucionador não achou o caminho: tente aproximar as peças.")).small().color(erro));
    } else {
        ui.label(RichText::new(tr("Em conflito com:")).small().color(erro));
        for (id, nome) in nomes {
            let r = ui.label(RichText::new(format!("  ✖ {nome}")).color(erro)).on_hover_text(tr("Tirar ou mudar esta restrição resolve o conflito"));
            if r.hovered() {
                *sob_mouse = Some(*id);
            }
        }
        if !p.conflito.minimo {
            ui.label(RichText::new(tr("A lista pode ter restrições a mais: a montagem é grande para testar uma a uma.")).small().weak());
        }
    }
}

/// Liga ou desliga o Animar. O cálculo sai da cópia da prévia: a montagem não muda.
fn alternar_animacao(app: &mut App, agora: f64) {
    let Some(st) = app.asm.as_mut() else { return };
    let base = st.asm.clone();
    let Some(pen) = st.cmd.as_mut() else { return };
    if pen.animacao.take().is_some() {
        st.fantasma_novo = true;
        return;
    }
    let Some(p) = pen.previa() else { return };
    match animar(&base, p, &pen.opcoes_animar) {
        Ok(q) => {
            pen.animacao = Some((q, agora));
            st.fantasma_novo = true;
        }
        Err(e) => app.toast(tr(&e).to_string(), true),
    }
}

/// `FENKRA_DEMO_ASM=bom_config` (B102): uma barra com duas configurações (Curta e Longa) usada três
/// vezes, uma delas com uma configuração que a peça não tem; a lista de peças abre mostrando o PN de
/// cada configuração e o erro da inexistente.
fn demo_bom_configuracoes(st: &mut AsmState) {
    use fenkra_core::configuracoes::{Coluna, Linha};
    use fenkra_core::{BoolOp, Document, Extent, Extrude, FeatureKind, OriginPlane, PlaneRef, Sketch, SketchData, Unit};
    let Some(dir) = st.dir() else { return };
    let mut d = Document::new("Barra");
    let _ = d.add_param("L", "100", Unit::Comprimento);
    let mut data = SketchData::default();
    {
        let mut al = || d.alloc();
        data.add_rect(&mut al, [0., 0.], [10., 10.], false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("L".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
    d.nova_coluna_config(Coluna::Parametro("L".into()));
    d.nova_coluna_config(Coluna::Pn);
    d.tabela.linhas.push(Linha { nome: "Curta".into(), celulas: vec!["50".into(), "B-050".into()] });
    d.tabela.linhas.push(Linha { nome: "Longa".into(), celulas: vec!["200".into(), "B-200".into()] });
    let _ = std::fs::write(dir.join("barra.fkp"), d.to_json());
    let mut a = Assembly::new("Conjunto de barras");
    for (i, cfg) in ["Curta", "Longa", "Longa", "Media"].iter().enumerate() {
        let id = a.add_component(&format!("Barra {}", i + 1), NodeKind::Part { path: "barra.fkp".into() }, Placement { origin: [0., 20. * i as f64, 0.], ..Default::default() }, None);
        a.definir_config(id, cfg);
    }
    st.asm = a;
    st.bom_open = Some(fenkra_montagem::bom::BomLevel::Topo);
    st.dirty = true;
}

/// `FENKRA_DEMO_ASM=cmd|junta|simbolos|manip|duzentas`: abre o comando ou os símbolos nas placas de exemplo.
pub fn demo(st: &mut AsmState, nome: &str) {
    if nome == "fixadores" {
        crate::asm_fixadores::demo(st);
        return;
    }
    if nome == "engrenagens" || nome == "transmissao" {
        // comando Gerar elemento de transmissão aberto (prévia 3D do par e cálculo ao vivo)
        let mut t = crate::transmissao_ui::TransUi::default();
        if let Ok(v) = std::env::var("FENKRA_DEMO_TRANS") {
            t.tipo = match v.as_str() {
                "helicoidais" => crate::transmissao_ui::Tipo::Helicoidais,
                "espinha" => crate::transmissao_ui::Tipo::EspinhaDePeixe,
                "conicas" => crate::transmissao_ui::Tipo::Conicas,
                "diferencial" => crate::transmissao_ui::Tipo::Diferencial,
                "semfim" => crate::transmissao_ui::Tipo::SemFim,
                "corrente" => crate::transmissao_ui::Tipo::Corrente,
                "correia" => crate::transmissao_ui::Tipo::Correia,
                _ => crate::transmissao_ui::Tipo::Retas,
            };
        }
        st.trans_open = Some(t);
        st.fantasma_novo = true;
        return;
    }
    if nome == "eixo" || nome == "engrenagens_classico" {
        // janela de engrenagens aberta (prévia do par) ou com o eixo gráfico ligado
        let mut g = crate::asm_ui::GearsUi::default();
        if nome == "eixo" {
            g.eixo_grafico = true;
            g.trecho_sel = Some(1);
            g.recursos = vec![
                fenkra_solidos::eixos_recursos::RecursoEixo { trecho: 1, borda: fenkra_solidos::eixos_recursos::Borda::Esquerda, posicao: 8., tipo: fenkra_solidos::eixos_recursos::TipoRecurso::Rasgo { comprimento: 30., rotacao: 0. } },
                fenkra_solidos::eixos_recursos::RecursoEixo { trecho: 1, borda: fenkra_solidos::eixos_recursos::Borda::Direita, posicao: 6., tipo: fenkra_solidos::eixos_recursos::TipoRecurso::Anel { largura: 1.5, profundidade: 0.8 } },
            ];
        }
        st.gears_open = Some(g);
        st.fantasma_novo = true;
        return;
    }
    if nome.starts_with("calculo") {
        crate::calculo_ui::demo(st, nome);
        return;
    }
    if nome.starts_with("chaveta") {
        crate::asm_chaveta::demo(st, nome);
        return;
    }
    if nome == "encaixe" {
        st.cmd = Some(Pendente::novo(Tipo::Junta(JointKind::Rigid)));
        crate::asm_encaixe::demo(st);
        return;
    }
    if nome == "repeticao" {
        crate::asm_repeticao::demo(st);
        return;
    }
    if nome == "bom_config" {
        demo_bom_configuracoes(st);
        return;
    }
    if let Some(n) = nome.strip_prefix("grade").and_then(|x| x.parse::<usize>().ok()) {
        // grade de peças iguais: `grade100` para olhar a cena com muitos corpos
        for k in 2..n {
            st.asm.add_component("Placa", NodeKind::Part { path: "placa.fkp".into() }, Placement { origin: [(k % 10) as f64 * 150., (k / 10) as f64 * 100., 0.], ..Default::default() }, None);
        }
        st.dirty = true;
        return;
    }
    if let Some(n) = nome.strip_prefix("variado").and_then(|x| x.parse::<usize>().ok()) {
        // peças todas diferentes para a placa de vídeo: nenhuma cópia divide malha com outra
        let Some(dir) = st.dir() else { return };
        for k in 2..n {
            let path = format!("placa_{k}.fkp");
            let _ = std::fs::copy(dir.join("placa.fkp"), dir.join(&path));
            st.asm.add_component("Placa", NodeKind::Part { path }, Placement { origin: [(k % 10) as f64 * 150., (k / 10) as f64 * 100., 0.], ..Default::default() }, None);
        }
        st.dirty = true;
        return;
    }
    if matches!(nome, "explosao" | "explosao_anim" | "exibicao") {
        crate::asm_exibicao::demo(st, nome);
        return;
    }
    if nome.starts_with("interf") {
        crate::asm_interf::demo(st, nome);
        return;
    }
    if nome.starts_with("inserir") {
        crate::asm_inserir::demo(st, nome);
        return;
    }
    let ids: Vec<fenkra_core::Id> = st.asm.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Group)).map(|n| n.id).collect();
    let [base, tampa] = ids[..] else { return };
    st.asm.node_mut(base).unwrap().fixed = true;
    let plano = |comp, z: f64, n: f64| MateRef { comp, geom: Geom::Plane { p: [40., 20., z], n: [0., 0., n] }, pick: Some([40., 20., z]) };
    if nome == "duzentas" {
        // grade de peças fixas com 200 restrições: a conta da tela, sem peso no solucionador
        let path = "placa.fkp".to_string();
        let mut grade = vec![base];
        for k in 1..40 {
            let origem = [(k % 8) as f64 * 150., (k / 8) as f64 * 100., 0.];
            grade.push(st.asm.add_component("Placa", NodeKind::Part { path: path.clone() }, Placement { origin: origem, ..Default::default() }, None));
        }
        st.asm.node_mut(tampa).unwrap().visible = false;
        for id in &grade {
            st.asm.node_mut(*id).unwrap().fixed = true;
        }
        let tipos = [MateKind::Parallel, MateKind::Contact, MateKind::Distance("12".into()), MateKind::Coincident { flip: false }];
        for k in 0..200usize {
            let (a, b) = (grade[k % 40], grade[(k * 7 + 1) % 40]);
            if a != b {
                st.asm.add_mate(tipos[k % 4].clone(), plano(a, 12., 1.), plano(b, 0., -1.));
            }
        }
        st.dirty = true;
    }
    let eixo = |comp| Elemento::novo(MateRef { comp, geom: Geom::Axis { p: [48., 24., 0.], d: [0., 0., 1.] }, pick: None }, Classe::Face);
    match nome {
        "cmd" => {
            let mut pen = Pendente::novo(Tipo::Restricao(MateKind::Contact));
            pen.c.receber(Elemento::novo(plano(base, 12., 1.), Classe::Face));
            pen.c.receber(Elemento::novo(plano(tampa, 0., -1.), Classe::Face));
            pen.textos = [vec![tr("Face plana").to_string()], vec![tr("Face plana").to_string()]];
            st.cmd = Some(pen);
            atualizar(st);
        }
        "junta" => {
            let mut pen = Pendente::novo(Tipo::JuntaAutomatica);
            pen.c.receber(eixo(base));
            pen.c.receber(eixo(tampa));
            pen.textos = [vec![tr("Eixo").to_string()], vec![tr("Eixo").to_string()]];
            st.cmd = Some(pen);
            atualizar(st);
        }
        "simbolos" => {
            st.asm.add_mate(MateKind::Distance("30".into()), plano(base, 12., 1.), plano(tampa, 0., -1.));
            st.solve();
            let fr = fenkra_montagem::kin::JFrame::from_axis([48., 24., 0.], [0., 0., 1.]);
            st.asm.add_joint(JointKind::Revolute, fenkra_montagem::kin::JRef { comp: base, frame: fr }, fenkra_montagem::kin::JRef { comp: tampa, frame: fr });
            st.solve();
        }
        "limites" => {
            // cilíndrica: arco e trilho na mesma junta, com um dos extremos desligado
            let fr = fenkra_montagem::kin::JFrame::from_axis([48., 24., 0.], [0., 0., 1.]);
            let id = st.asm.add_joint(JointKind::Cylindrical, fenkra_montagem::kin::JRef { comp: base, frame: fr }, fenkra_montagem::kin::JRef { comp: tampa, frame: fr });
            if let Some(j) = st.asm.joints.iter_mut().find(|j| j.id == id) {
                j.limites.angulo = kin::Faixa::nova(-40., 75.);
                j.limites.curso = kin::Faixa::nova(0., 30.);
                j.limites.curso.min.ligado = false;
            }
            st.solve();
            st.sel = Some(id);
        }
        "limites_cmd" => {
            let mut pen = Pendente::novo(Tipo::Junta(JointKind::Revolute));
            pen.c.receber(eixo(base));
            pen.c.receber(eixo(tampa));
            pen.c.limites.angulo = kin::Faixa::nova(-30., 110.);
            pen.textos = [vec![tr("Eixo").to_string()], vec![tr("Eixo").to_string()]];
            st.cmd = Some(pen);
            atualizar(st);
        }
        "conflito" => {
            // duas distâncias já gravadas e a proposta pedindo outra: ela mesma e a primeira brigam
            let d = st.asm.add_mate(MateKind::Distance("30".into()), plano(base, 12., 1.), plano(tampa, 0., -1.));
            if let Some(m) = st.asm.mates.iter_mut().find(|m| m.id == d) {
                m.sentido = fenkra_montagem::Sentido::Oposto;
            }
            st.solve();
            let mut pen = Pendente::novo(Tipo::Restricao(MateKind::Contact));
            pen.c.receber(Elemento::novo(plano(base, 12., 1.), Classe::Face));
            pen.c.receber(Elemento::novo(plano(tampa, 0., -1.), Classe::Face));
            pen.textos = [vec![tr("Face plana").to_string()], vec![tr("Face plana").to_string()]];
            st.cmd = Some(pen);
            atualizar(st);
        }
        "manip" => {
            let fr = fenkra_montagem::kin::JFrame::from_axis([48., 24., 0.], [0., 0., 1.]);
            st.asm.add_joint(JointKind::Revolute, fenkra_montagem::kin::JRef { comp: base, frame: fr }, fenkra_montagem::kin::JRef { comp: tampa, frame: fr });
            st.solve();
            st.sel = Some(tampa);
        }
        _ => {}
    }
}

/// Peças da prévia como fantasma, na posição final. Em vermelho se a proposta não tem solução.
pub fn na_cena(st: Option<&AsmState>, v: &egui::Visuals, t: f64, cena: &mut crate::render::Scene) {
    let Some(st) = st else { return };
    let (Some(pen), Some(vista)) = (st.cmd.as_ref(), st.view.as_ref()) else { return };
    let Some(p) = pen.previa() else { return };
    let c = if p.falhou() { crate::theme::st_err() } else { crate::theme::accent(v) };
    for mv in pen.movidos_em(t) {
        for b in vista.bodies.iter().filter(|b| b.collector == mv.comp) {
            let m = fenkra_montagem::view::transformed(&b.mesh, &mv.delta());
            for fi in 0..m.face_ranges.len() {
                crate::previa::face_fantasma(&mut cena.ghost, &m, fi, [c.r(), c.g(), c.b()]);
            }
        }
    }
}

/// Ponto de ancoragem e direção do elemento, no sistema da peça.
fn ancora(e: &Elemento) -> ([f64; 3], Option<[f64; 3]>) {
    match e.r.geom {
        Geom::Plane { p, n } => (e.r.pick.unwrap_or(p), Some(n)),
        Geom::Axis { p, d } => (p, Some(d)),
        Geom::Point(p) => (p, None),
    }
}

fn seta(painter: &egui::Painter, rect: egui::Rect, cam: &crate::render::Camera, e: &Elemento, pl: &Placement, c: Color32, fantasma: bool) -> Option<egui::Pos2> {
    let v3 = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let (o, d) = ancora(e);
    let o = pl.point(o);
    let base = cam.project(rect, v3(o))?;
    let c = if fantasma { c.gamma_multiply(0.7) } else { c };
    if let Some(d) = d {
        if let Some((_, na_tela, _)) = crate::gizmo::screen_axis(cam, rect, v3(o), v3(unit(pl.dir(d)))) {
            crate::gizmo::arrow(painter, base, base + na_tela * 56., c);
        }
    }
    painter.circle_filled(base, 4., c);
    Some(base)
}

/// Seleção e setas de normal do comando aberto: a de cada elemento na posição de agora e, na peça
/// que se move, também na posição final.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    if let Some(st) = app.asm.as_mut().filter(|s| s.cmd.as_ref().is_some_and(|c| c.animacao.is_some())) {
        // cada quadro do Animar é um fantasma novo
        st.fantasma_novo = true;
        ui.ctx().request_repaint();
    }
    let Some(st) = app.asm.as_ref() else { return };
    let Some(pen) = st.cmd.as_ref() else { return };
    let painter = ui.painter_at(rect);
    let v = ui.visuals().clone();
    let previa = pen.previa();
    let mut alvo = None;
    for k in 0..2 {
        if let Some(r) = app.result.as_ref() {
            for p in &pen.picks[k] {
                crate::picking::draw(&painter, r, cam, rect, p, true);
            }
        }
        let Some(e) = &pen.c.caixas[k] else { continue };
        let Some(pl) = st.asm.lugar(e.r.comp) else { continue };
        let c = cor(&v, k);
        alvo = seta(&painter, rect, cam, e, &pl, c, false).or(alvo);
        if let Some(mv) = previa.and_then(|p| p.movidos.iter().find(|m| m.comp == e.r.comp)) {
            seta(&painter, rect, cam, e, &mv.depois, c, true);
        }
    }
    if let (Some(a), true) = (alvo, previa.is_some_and(|p| p.falhou())) {
        let t = painter.layout_no_wrap(tr("Sem solução").to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), crate::theme::st_err());
        painter.galley(a + egui::vec2(10., 10.), t, crate::theme::st_err());
    }
    if let Some(p) = previa.filter(|p| p.falhou()) {
        conflito_no_modelo(&painter, rect, cam, &st.asm, &p.conflito.envolvidas, pen.conflito_sob_mouse);
    }
}

/// Liga as duas referências de cada restrição do conflito, com o nome em vermelho. O símbolo sai da
/// montagem de verdade: a cópia da prévia tem as peças num meio-termo que não vale nada.
fn conflito_no_modelo(painter: &egui::Painter, rect: egui::Rect, cam: &crate::render::Camera, a: &Assembly, ids: &[fenkra_core::Id], sob_mouse: Option<fenkra_core::Id>) {
    let v3 = |p: [f64; 3]| glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let erro = crate::theme::st_err();
    for s in fenkra_montagem::simbolos::simbolos(a, ids).iter().filter(|s| ids.contains(&s.id)) {
        let (Some(pa), Some(pb)) = (cam.project(rect, v3(s.a)), cam.project(rect, v3(s.b))) else { continue };
        let quente = sob_mouse == Some(s.id);
        painter.line_segment([pa, pb], egui::Stroke::new(if quente { 3.4 } else { 2. }, erro));
        painter.circle_filled(pa, 4., erro);
        painter.circle_filled(pb, 4., erro);
        let g = painter.layout_no_wrap(a.name_of(s.id), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), erro);
        painter.galley(pa + (pb - pa) / 2. + egui::vec2(8., -16.), g, erro);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modos_e_etapas_tem_nome_em_portugues_e_traducao() {
        let mut nomes: Vec<&str> = RESTRICOES.iter().map(|(m, _, _)| tipo_da_restricao(*m).label()).collect();
        nomes.extend((0..9).map(|k| tipo_da_junta(k).label()));
        for n in nomes {
            assert!(n.chars().all(|c| c.is_alphanumeric() || " ()".contains(c)), "{n}");
            assert!(crate::i18n::tem_traducao(n), "sem tradução: {n}");
            assert!(!["Coincident", "Contact", "Distance", "Angle", "Parallel", "Perpendicular", "Revolute", "Prismatic", "Cylindrical", "Screw", "Spherical", "Rigid", "Cam"].contains(&n), "nome interno: {n}");
        }
    }

    #[test]
    fn nenhum_texto_imprime_o_enum() {
        let fonte = concat!(include_str!("asm_cmd.rs"), include_str!("asm_ui.rs"));
        assert!(!fonte.contains(concat!("{m", ":?}")), "imprime o nome interno da restrição");
    }

    #[test]
    fn junta_pendente_tem_aviso_de_etapa_fixo() {
        let mut c = Comando::novo(tipo_da_junta(0));
        assert!(etapa_texto(&c).contains("primeiro"));
        c.ativa = 1;
        assert!(etapa_texto(&c).contains("segundo"));
        assert!(crate::i18n::tem_traducao(etapa_texto(&c)));
    }

    /// Peça de exemplo como vista da montagem: base fixa, tampa solta e o comando de contato completo.
    fn estado_com_contato() -> (AsmState, fenkra_core::Id) {
        let mut a = Assembly::new("par");
        let base = a.add_component("base", NodeKind::Part { path: "a.fkp".into() }, Placement::default(), None);
        a.node_mut(base).unwrap().fixed = true;
        let tampa = a.add_component("tampa", NodeKind::Part { path: "b.fkp".into() }, Placement { origin: [0., 0., 30.], ..Default::default() }, None);
        let mut st = AsmState::new(a, None);
        let mut r = fenkra_solidos::recompute(&fenkra_solidos::example_document());
        r.bodies.truncate(1);
        r.bodies[0].collector = tampa;
        st.view = Some(r);
        let mut pen = Pendente::novo(Tipo::Restricao(MateKind::Contact));
        let face = |comp, z: f64, n: f64| Elemento::novo(MateRef { comp, geom: Geom::Plane { p: [0., 0., z], n: [0., 0., n] }, pick: None }, Classe::Face);
        pen.c.receber(face(base, 10., 1.));
        pen.c.receber(face(tampa, 0., -1.));
        st.cmd = Some(pen);
        (st, tampa)
    }

    #[test]
    fn fantasma_aparece_na_posicao_final_e_a_montagem_fica() {
        let (mut st, tampa) = estado_com_contato();
        let antes = st.asm.clone();
        atualizar(&mut st);
        assert!(st.fantasma_novo, "a cena precisa saber da prévia");
        let v = egui::Visuals::dark();
        let mut cena = crate::render::Scene::default();
        na_cena(Some(&st), &v, 0., &mut cena);
        assert!(!cena.ghost.is_empty());
        let topo = |pos: &mut dyn Iterator<Item = f32>| pos.fold(f32::MIN, f32::max);
        let malha = &st.view.as_ref().unwrap().bodies[0].mesh;
        let z_vista = topo(&mut malha.positions.chunks_exact(3).map(|p| p[2]));
        let z_fantasma = topo(&mut cena.ghost.iter().map(|g| g.pos[2]));
        assert!((z_vista - z_fantasma - 20.).abs() < 1e-3, "a tampa desce 20: {z_vista} {z_fantasma}");
        assert!(cena.ghost.iter().all(|g| g.col[3] < 1.), "translúcido");
        assert_eq!(st.asm, antes, "cancelar não tem o que desfazer");
        assert_eq!(st.asm.node(tampa).unwrap().placement.origin[2], 30.);
    }

    #[test]
    fn sem_solucao_o_fantasma_fica_na_cor_de_erro() {
        let (mut st, tampa) = estado_com_contato();
        // a tampa pode girar, mas um contato com a base numa altura impossível: fixa em outra face
        let extra = st.asm.add_mate(
            MateKind::Distance("50".into()),
            MateRef { comp: st.asm.nodes[0].id, geom: Geom::Plane { p: [0., 0., 10.], n: [0., 0., 1.] }, pick: None },
            MateRef { comp: tampa, geom: Geom::Plane { p: [0.; 3], n: [0., 0., -1.] }, pick: None },
        );
        assert!(st.asm.mates.iter().any(|m| m.id == extra));
        st.report = solve::solve(&mut st.asm);
        atualizar(&mut st);
        let p = st.cmd.as_ref().unwrap().previa().unwrap();
        assert!(p.falhou());
        assert_eq!(p.conflito.envolvidas, vec![extra], "aponta a restrição que briga com a proposta");
        assert!(st.em_conflito(extra));
        let mut cena = crate::render::Scene::default();
        na_cena(Some(&st), &egui::Visuals::dark(), 0., &mut cena);
        let erro = crate::theme::st_err();
        assert!(cena.ghost.iter().all(|g| (g.col[0] - erro.r() as f32 / 255.).abs() < 1e-6));
    }

    #[test]
    fn animar_troca_o_fantasma_a_cada_quadro_sem_tocar_a_montagem() {
        let (mut st, tampa) = estado_com_contato();
        let base = st.asm.nodes[0].id;
        let eixo = |comp| Elemento::novo(MateRef { comp, geom: Geom::Axis { p: [20., 0., 0.], d: [0., 0., 1.] }, pick: None }, Classe::Face);
        let mut pen = Pendente::novo(Tipo::JuntaAutomatica);
        pen.c.receber(eixo(base));
        pen.c.receber(eixo(tampa));
        st.cmd = Some(pen);
        atualizar(&mut st);
        let antes = st.asm.clone();
        let quadros = {
            let pen = st.cmd.as_ref().unwrap();
            let p = pen.previa().unwrap();
            assert!(matches!(p.copia.joints[0].kind, JointKind::Revolute), "dois furos: revolução");
            animar(&st.asm, p, &pen.opcoes_animar).unwrap()
        };
        st.cmd.as_mut().unwrap().animacao = Some((quadros, 0.));
        let quadro = |t: f64| {
            let mut cena = crate::render::Scene::default();
            na_cena(Some(&st), &egui::Visuals::dark(), t, &mut cena);
            cena.ghost.iter().map(|g| g.pos[1]).fold(f32::MIN, f32::max)
        };
        let (y0, y1) = (quadro(1. / QUADROS_POR_SEGUNDO), quadro(18. / QUADROS_POR_SEGUNDO));
        assert!((y0 - y1).abs() > 1., "a peça anda entre os quadros: {y0} {y1}");
        assert_eq!(st.asm, antes);
    }

    #[test]
    fn trocar_a_proposta_para_a_animacao() {
        let (mut st, _) = estado_com_contato();
        atualizar(&mut st);
        let pen = st.cmd.as_mut().unwrap();
        pen.animacao = Some((vec![vec![]], 0.));
        pen.c.valor = Some("3".into());
        atualizar(&mut st);
        assert!(st.cmd.as_ref().unwrap().animacao.is_none());
    }

    #[test]
    fn modo_da_barra_vira_tipo_certo() {
        assert_eq!(tipo_da_restricao(MateCmd::Auto), Tipo::Automatica);
        assert_eq!(tipo_da_restricao(MateCmd::AlignOnce), Tipo::Alinhar);
        assert!(matches!(tipo_da_restricao(MateCmd::Contact), Tipo::Restricao(MateKind::Contact)));
        assert!(matches!(tipo_da_junta(7), Tipo::Junta(JointKind::PointCurve { .. })));
        assert_eq!(tipo_da_junta(AUTOMATICA), Tipo::JuntaAutomatica);
    }
}
