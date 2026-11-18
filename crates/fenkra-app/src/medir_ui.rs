// SPDX-License-Identifier: LGPL-2.1-or-later
//! B509: comando Medir (paquímetro). Dois modos no mesmo painel:
//!
//! - **Medir item**: um elemento (vértice, aresta, face) e o que ele tem: comprimento, raio,
//!   diâmetro, área, perímetro, ângulo do arco, centro;
//! - **Medir entre**: duas listas e o que há entre elas: distância mínima, ângulo, plano a plano,
//!   eixo a eixo, distância ao eixo e, com "ilimitado", plano e reta tratados como infinitos.
//!
//! O clique no 3D entra direto na lista ativa (medida única: o primeiro clique vai para a primeira
//! lista, o segundo para a segunda; com "várias por lista" o clique soma na lista ativa). O resultado
//! aparece no painel e num rótulo simples no 3D (não é anotação). "Manter medida" grava a medida na
//! pasta Medidas da árvore: ela guarda as chaves v2 dos elementos e é refeita a cada cálculo; se um
//! elemento some, ela fica órfã e sinalizada, sem valor velho.
//!
//! O cálculo fica em `medir.rs`; aqui só a interface.

use crate::app::{fmt, App};
use crate::i18n::tr;
use crate::medir::{self, Leitura, Valor};
use crate::picking::Pick;
use crate::render::Camera;
use eframe::egui::{self, RichText};
use fenkra_core::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Estado do comando aberto.
#[derive(Clone, Default)]
pub struct Medir {
    pub modo: ModoMedida,
    /// (referência, rótulo legível) de cada lista
    pub a: Vec<(RefMedida, String)>,
    pub b: Vec<(RefMedida, String)>,
    /// lista que recebe o próximo clique: false = primeira, true = segunda
    pub na_segunda: bool,
    /// clique soma na lista ativa em vez de trocar (multisseleção por lista)
    pub varios: bool,
    pub infinito: bool,
    pub manter: bool,
    /// seleção do 3D já absorvida pelas listas
    vistos: Vec<Pick>,
}

impl Medir {
    pub fn novo(modo: ModoMedida) -> Self {
        Medir { modo, ..Default::default() }
    }

    /// Recebe um elemento clicado (já convertido em referência).
    pub fn receber(&mut self, r: RefMedida, rotulo: String) {
        match self.modo {
            ModoMedida::Item => self.a = vec![(r, rotulo)],
            ModoMedida::Entre => {
                let lista = if self.na_segunda { &mut self.b } else { &mut self.a };
                if self.varios {
                    lista.push((r, rotulo));
                } else {
                    *lista = vec![(r, rotulo)];
                    // medida única: a primeira lista preenchida passa a vez para a segunda
                    self.na_segunda = !self.na_segunda;
                }
            }
        }
    }

    /// A medida como ela seria gravada (também serve para calcular o resultado ao vivo).
    pub fn como_medida(&self) -> Medida {
        let refs = |v: &[(RefMedida, String)]| v.iter().map(|(r, _)| r.clone()).collect();
        Medida { id: Id(0), nome: String::new(), modo: self.modo, a: refs(&self.a), b: if self.modo == ModoMedida::Entre { refs(&self.b) } else { vec![] }, infinito: self.infinito, campos: vec![] }
    }

    fn pronta(&self) -> bool {
        !self.a.is_empty() && (self.modo == ModoMedida::Item || !self.b.is_empty())
    }
}

/// Botão da barra de Análise.
pub fn item() -> crate::toolbar::Item {
    crate::toolbar::Item {
        icon: "medir",
        name: "Medir",
        brief: "Medir item ou entre elementos (paquímetro)",
        advanced: "Clique um elemento para ver comprimento, raio, diâmetro, área, ângulo e centro; ou dois para a distância mínima, o ângulo, plano a plano e eixo a eixo. Manter medida põe a medida na árvore, refeita a cada mudança do modelo.",
        act: crate::toolbar::Act::Cmd(crate::app::Cmd::Medir),
    }
}

/// Abre o comando (Medir item; o modo troca no painel).
pub fn abrir(app: &mut App) {
    let mut m = Medir::novo(ModoMedida::Item);
    // o que já está selecionado entra como primeiro clique
    let picks = app.picks.clone();
    if picks.len() >= 2 {
        m.modo = ModoMedida::Entre;
    }
    for p in &picks {
        if let Some((r, t)) = ref_de_pick(app, p) {
            m.receber(r, t);
        }
    }
    m.vistos = picks;
    app.medir = Some(m);
}

pub fn fechar(app: &mut App) {
    app.medir = None;
}

/// Seleção do 3D virando referência de medida, com a chave v2 do elemento (ADR 0010).
pub fn ref_de_pick(app: &App, p: &Pick) -> Option<(RefMedida, String)> {
    let rotulo = rotulo_pick(&app.open.doc, p);
    let r = match *p {
        Pick::Face { point, .. } => RefMedida { elem: ElemMedida::Face, ponto: point.map(|v| v as f64), face: app.face_key_of(Some(p)), aresta: None },
        Pick::Edge { body, edge, mid } => RefMedida { elem: ElemMedida::Aresta, ponto: mid.map(|v| v as f64), face: None, aresta: app.edge_key_of(body, edge) },
        Pick::Vertex { body, pos } => {
            let pos = pos.map(|v| v as f64);
            let aresta = match app.vertex_ref(body, pos) {
                PointRef::Vertex { edge, .. } => Some(edge),
                _ => None,
            };
            RefMedida { elem: ElemMedida::Vertice, ponto: pos, face: None, aresta }
        }
        _ => return None,
    };
    Some((r, rotulo))
}

/// Nome curto do elemento, no idioma da interface.
pub fn rotulo_pick(doc: &Document, p: &Pick) -> String {
    match p {
        Pick::Face { owner, .. } => format!("{} · {}", tr("Face"), doc.name_of(*owner)),
        Pick::Edge { edge, .. } => format!("{} {}", tr("Aresta"), edge + 1),
        Pick::Vertex { pos, .. } => format!("{} ({}; {}; {})", tr("Vértice"), fmt(pos[0] as f64), fmt(pos[1] as f64), fmt(pos[2] as f64)),
        outro => outro.label(doc),
    }
}

/// Puxa para as listas o que a pessoa clicou no 3D desde o último quadro.
fn absorver(app: &mut App) {
    let Some(m) = app.medir.as_ref() else { return };
    let novos: Vec<Pick> = app.picks.iter().filter(|p| !m.vistos.contains(p)).copied().collect();
    let refs: Vec<(RefMedida, String)> = novos.iter().filter_map(|p| ref_de_pick(app, p)).collect();
    let picks = app.picks.clone();
    let Some(m) = app.medir.as_mut() else { return };
    for (r, t) in refs {
        m.receber(r, t);
    }
    m.vistos = picks;
}

// ------------------------------------------------------------------ cálculo com memória

thread_local! {
    static MEMO: RefCell<(u64, HashMap<u64, Leitura>)> = RefCell::new((0, HashMap::new()));
}

fn chave_resultado(r: &fenkra_solidos::RecomputeResult) -> u64 {
    (r as *const _ as u64).wrapping_mul(31) ^ r.elapsed_ms.to_bits() ^ (r.bodies.len() as u64) << 48
}

fn hash_de(m: &Medida) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_string(&(m.modo, &m.a, &m.b, m.infinito, &m.campos)).unwrap_or_default().hash(&mut h);
    h.finish()
}

/// Leitura da medida no resultado atual (refeita só quando o resultado ou a medida muda).
pub fn ler(r: &fenkra_solidos::RecomputeResult, m: &Medida) -> Leitura {
    let k = chave_resultado(r);
    let h = hash_de(m);
    MEMO.with(|c| {
        let mut c = c.borrow_mut();
        if c.0 != k {
            *c = (k, HashMap::new());
        }
        c.1.entry(h).or_insert_with(|| medir::avaliar(r, m)).clone()
    })
}

fn texto_valor(v: &Valor) -> String {
    match v {
        Valor::Num(x, u) => format!("{} {}", fmt(*x), u.texto()),
        Valor::Ponto(p) => format!("({}; {}; {}) mm", fmt(p[0]), fmt(p[1]), fmt(p[2])),
    }
}

/// Texto curto do rótulo no 3D e na árvore: o valor principal.
pub fn resumo(l: &Leitura) -> String {
    if l.orfa() {
        return tr("órfã: referência perdida").to_string();
    }
    match l.principal().or(l.linhas.first()) {
        Some(x) => format!("{} {}", tr(x.rotulo), texto_valor(&x.valor)),
        None => l.falta.map(|f| tr(f).to_string()).unwrap_or_default(),
    }
}

// ------------------------------------------------------------------ painel

/// O painel do comando está na coluna (nada mais aberto por cima dele).
pub fn no_painel(app: &App) -> bool {
    app.medir.is_some() && app.session.is_none() && app.sketch.is_none() && !app.in_asm()
}

fn lista(ui: &mut egui::Ui, titulo: &str, itens: &mut Vec<(RefMedida, String)>, ativa: bool) -> bool {
    let mut clicou = false;
    let cor = if ativa { crate::theme::accent(ui.visuals()) } else { ui.visuals().text_color() };
    ui.horizontal(|ui| {
        let r = ui.selectable_label(ativa, RichText::new(titulo).strong().color(cor)).on_hover_text(tr("O próximo clique no 3D entra nesta lista."));
        clicou = r.clicked();
        if !itens.is_empty() && ui.small_button("✕").on_hover_text(tr("Limpar a lista")).clicked() {
            itens.clear();
            clicou = true;
        }
    });
    if itens.is_empty() {
        ui.label(RichText::new(tr("clique um vértice, uma aresta ou uma face")).weak().small());
    }
    let mut tirar = None;
    for (i, (_, t)) in itens.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(t);
            if ui.small_button("−").on_hover_text(tr("Tirar da lista")).clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        itens.remove(i);
    }
    clicou
}

fn resultado(ui: &mut egui::Ui, l: &Leitura, sal: &str) {
    if let Some(f) = l.falta {
        ui.label(RichText::new(tr(f)).weak());
        return;
    }
    egui::Grid::new(sal).num_columns(3).striped(true).show(ui, |ui| {
        for x in &l.linhas {
            ui.label(tr(x.rotulo));
            let t = texto_valor(&x.valor);
            ui.label(RichText::new(&t).strong());
            if ui.small_button("⧉").on_hover_text(tr("Copiar o valor")).clicked() {
                ui.ctx().copy_text(t);
            }
            ui.end_row();
        }
    });
}

/// Painel do comando Medir.
pub fn painel(app: &mut App, ui: &mut egui::Ui) {
    absorver(app);
    let Some(mut m) = app.medir.clone() else { return };
    ui.horizontal(|ui| {
        if ui.selectable_label(m.modo == ModoMedida::Item, tr("Medir item")).on_hover_text(tr("Um elemento: comprimento, raio, diâmetro, área, ângulo, centro.")).clicked() && m.modo != ModoMedida::Item {
            m.modo = ModoMedida::Item;
            m.b.clear();
            m.a.truncate(1);
        }
        if ui.selectable_label(m.modo == ModoMedida::Entre, tr("Medir entre")).on_hover_text(tr("Duas listas: distância mínima, ângulo, plano a plano, eixo.")).clicked() && m.modo != ModoMedida::Entre {
            m.modo = ModoMedida::Entre;
            m.na_segunda = !m.a.is_empty();
        }
    });
    ui.add_space(4.);
    match m.modo {
        ModoMedida::Item => {
            lista(ui, tr("Elemento"), &mut m.a, true);
        }
        ModoMedida::Entre => {
            if lista(ui, tr("Primeira lista"), &mut m.a, !m.na_segunda) {
                m.na_segunda = false;
            }
            if lista(ui, tr("Segunda lista"), &mut m.b, m.na_segunda) {
                m.na_segunda = true;
            }
            ui.checkbox(&mut m.varios, tr("Várias por lista")).on_hover_text(tr("O clique soma na lista ativa; a distância é a menor entre as duas listas."));
            ui.checkbox(&mut m.infinito, tr("Plano e reta ilimitados")).on_hover_text(tr("Trata o plano da face e a reta da aresta como infinitos (geometria prolongada)."));
        }
    }
    ui.separator();
    if m.pronta() {
        if let Some(r) = &app.result {
            let l = ler(r, &m.como_medida());
            resultado(ui, &l, "medir_resultado");
        }
    } else {
        ui.label(RichText::new(tr(if m.modo == ModoMedida::Item { "Escolha um elemento." } else { "Escolha os elementos das duas listas." })).weak());
    }
    ui.separator();
    ui.checkbox(&mut m.manter, tr("Manter medida")).on_hover_text(tr("Cria a medida na pasta Medidas da árvore. Ela é refeita a cada mudança do modelo e fica órfã, sinalizada, se um elemento sumir."));
    let mut concluir = false;
    let mut fechar_ja = false;
    ui.horizontal(|ui| {
        let ok = ui.add_enabled(m.pronta() || !m.manter, egui::Button::new(tr("Concluir"))).on_hover_text(tr("Fecha o comando; com Manter medida, grava a medida na árvore."));
        concluir = ok.clicked();
        fechar_ja = ui.button(tr("Fechar")).clicked();
    });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !ui.ctx().egui_wants_keyboard_input() {
        fechar_ja = true;
    }
    if concluir && m.manter && m.pronta() {
        let g = m.como_medida();
        let d = &mut app.open.doc;
        let id = d.add_medida(g.modo, g.a, g.b, g.infinito);
        let nome = d.medida(id).map(|x| x.nome.clone()).unwrap_or_default();
        app.toast(format!("{} {nome}", tr("Medida criada:")), false);
        app.medir = None;
        return;
    }
    if concluir || fechar_ja {
        app.medir = None;
        return;
    }
    app.medir = Some(m);
}

// ------------------------------------------------------------------ 3D

fn caixa(painter: &egui::Painter, v: &egui::Visuals, at: egui::Pos2, texto: &str, alerta: bool) {
    let fonte = egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.));
    let cor_txt = if alerta { crate::theme::st_warn() } else { v.text_color() };
    let gal = painter.layout_no_wrap(texto.to_string(), fonte, cor_txt);
    let r = egui::Rect::from_min_size(at + egui::vec2(10., -gal.size().y - 10.), gal.size()).expand(4.);
    painter.rect_filled(r, 3., crate::theme::com_alfa(v.extreme_bg_color, 235));
    painter.rect_stroke(r, 3., egui::Stroke::new(1., if alerta { crate::theme::st_warn() } else { crate::theme::accent(v) }), egui::StrokeKind::Inside);
    painter.galley(r.min + egui::vec2(4., 4.), gal, cor_txt);
    painter.line_segment([at, r.left_bottom()], egui::Stroke::new(1., v.weak_text_color()));
}

fn vp(p: [f64; 3]) -> glam::Vec3 {
    glam::Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Rótulos no 3D: a medida do comando aberto e as medidas mantidas na árvore.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: egui::Rect, cam: &Camera) {
    aplicar_pendentes(app, ui.ctx());
    let Some(r) = &app.result else { return };
    if app.in_asm() {
        return;
    }
    let painter = ui.painter_at(rect);
    let v = ui.visuals().clone();
    let linha = egui::Stroke::new(1.6, crate::theme::accent(&v));
    let desenhar = |l: &Leitura, nome: Option<&str>| {
        if let Some((p, q)) = l.apoio {
            if let (Some(a), Some(b)) = (cam.project(rect, vp(p)), cam.project(rect, vp(q))) {
                painter.line_segment([a, b], linha);
                painter.circle_filled(a, 3., linha.color);
                painter.circle_filled(b, 3., linha.color);
            }
        }
        let Some(at) = l.ancora.and_then(|c| cam.project(rect, vp(c))) else { return };
        let corpo = resumo(l);
        let texto = match nome {
            Some(n) => format!("{n}: {corpo}"),
            None => corpo,
        };
        caixa(&painter, &v, at, &texto, l.orfa());
    };
    if let Some(m) = app.medir.as_ref().filter(|m| m.pronta()) {
        desenhar(&ler(r, &m.como_medida()), None);
    }
    for m in &app.open.doc.medidas {
        let l = ler(r, m);
        // a órfã não tem onde se ancorar: o aviso fica na árvore
        desenhar(&l, Some(&m.nome));
    }
}

// ------------------------------------------------------------------ árvore

#[derive(Clone, Copy, Debug)]
enum AcaoArvore {
    Remover(Id),
    Parametro(Id),
}

fn chave_pendentes() -> egui::Id {
    egui::Id::unique("medidas_pendentes_b509")
}

/// Pasta Medidas da árvore (só aparece com alguma medida).
pub fn no_da_arvore(app: &App, ui: &mut egui::Ui, doc: &Document) {
    if doc.medidas.is_empty() {
        return;
    }
    let mut acoes: Vec<AcaoArvore> = vec![];
    let _ = crate::tree_ui::grupo(ui, "arvore_medidas", Some("medida"), format!("{} ({})", tr("Medidas mantidas"), doc.medidas.len()), true).corpo(ui, |ui| {
        for m in &doc.medidas {
            let l = app.result.as_ref().map(|r| ler(r, m)).unwrap_or_default();
            let orfa = l.orfa();
            let texto = format!("{} = {}", m.nome, resumo(&l));
            let rt = if orfa { RichText::new(texto).color(crate::theme::st_warn()) } else { RichText::new(texto) };
            let resp = crate::tree_ui::linha(ui, if orfa { "medida_orfa" } else { "medida" }, false, rt);
            let resp = if orfa {
                resp.on_hover_text(tr("Medida órfã: um elemento medido não existe mais no modelo. Apague-a ou meça de novo."))
            } else {
                resp.on_hover_text(tr("Medida associativa: refeita a cada mudança do modelo."))
            };
            resp.context_menu(|ui| {
                if ui.add_enabled(!orfa, egui::Button::new(tr("Criar parâmetro com o valor"))).clicked() {
                    acoes.push(AcaoArvore::Parametro(m.id));
                    ui.close();
                }
                if ui.button(tr("Apagar")).clicked() {
                    acoes.push(AcaoArvore::Remover(m.id));
                    ui.close();
                }
            });
        }
    });
    if !acoes.is_empty() {
        ui.ctx().data_mut(|d| d.get_temp_mut_or_default::<Vec<AcaoArvore>>(chave_pendentes()).extend(acoes));
    }
}

fn aplicar_pendentes(app: &mut App, ctx: &egui::Context) {
    let acoes: Vec<AcaoArvore> = ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<Vec<AcaoArvore>>(chave_pendentes())));
    for a in acoes {
        match a {
            AcaoArvore::Remover(id) => {
                app.open.doc.remover_medida(id);
            }
            AcaoArvore::Parametro(id) => {
                let Some(m) = app.open.doc.medida(id).cloned() else { continue };
                let Some(l) = app.result.as_ref().map(|r| ler(r, &m)) else { continue };
                let Some(x) = l.principal() else { continue };
                let Valor::Num(v, u) = x.valor else { continue };
                let unidade = match u {
                    medir::Unidade::Mm => Unit::Comprimento,
                    medir::Unidade::Grau => Unit::Angulo,
                    medir::Unidade::Mm2 => Unit::Numero,
                };
                let d = &mut app.open.doc;
                let n = d.unique_name(&format!("{}_", m.nome));
                // valor fixo: o parâmetro não segue a medida (evita ciclo parâmetro → forma → medida)
                let _ = d.add_param(&n, &format!("{v}"), unidade);
                app.toast(format!("{} {n} = {}", tr("Parâmetro criado:"), fmt(v)), false);
            }
        }
    }
}
