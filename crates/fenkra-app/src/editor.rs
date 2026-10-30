//! Painel à direita: edição no lugar do objeto selecionado (RF-UI-002, RF-NUC-020).
//! Toda alteração modifica o objeto existente; o recompute é disparado automaticamente.

use crate::app::{fmt, App, Sel, MATERIALS};
use crate::scene::{plane_name, plane_ref_name};
use eframe::egui::{self, Color32, RichText};
use fenkra_core::*;
use fenkra_solidos::hardware;
use fenkra_solidos::Status;
use std::collections::HashMap;
use crate::i18n::tr;

type Vars = HashMap<String, f64>;

/// Ícone de um parâmetro pelo que ele mede (comprimento, ângulo, raio…), pelo nome.
pub(crate) fn param_icon(label: &str) -> &'static str {
    let l = label.to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    if has(&["ângulo", "angulo", "angle", "inclina", "giro", "rota"]) {
        "∠"
    } else if has(&["diâmetro", "diametro", "diameter", "⌀", "ø"]) {
        "⌀"
    } else if has(&["raio", "radius", "arredond", "filete"]) {
        "◜"
    } else if has(&["espessura", "thickness", "parede", "chapa"]) {
        "▭"
    } else if has(&["profundidade", "depth", "fundo"]) {
        "↧"
    } else if has(&["altura", "height", "elevação"]) {
        "↕"
    } else if has(&["largura", "width", "boca"]) {
        "⇿"
    } else if has(&["folga", "gap", "recuo", "alívio", "alivio", "relief", "clearance"]) {
        "⇹"
    } else if has(&["deslocamento", "offset", "distância", "distancia", "afast", "desvio"]) {
        "⇥"
    } else if has(&["passo", "pitch", "espaçamento", "espacamento", "intervalo"]) {
        "⋯"
    } else if has(&["quantidade", "número", "numero", "count", "instâncias", "cópias", "voltas"]) {
        "#"
    } else if has(&["fator", "factor", "razão", "ratio", "escala", "scale"]) || l == "k" {
        "ƒ"
    } else if has(&["comprimento", "length", "extensão", "extensao", "tamanho", "lado", "aba", "bainha"]) {
        "↔"
    } else if has(&["peso", "massa", "densidade"]) {
        "⚖"
    } else if has(&["tempo", "velocidade", "duração"]) {
        "⏱"
    } else {
        "◦"
    }
}

/// Ícone de uma seção/aba do painel, pelo título.
pub(crate) fn section_icon(title: &str) -> &'static str {
    let l = title.to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    if has(&["perfil", "profile", "sketch", "guia"]) {
        "✏"
    } else if has(&["rosca", "fixador", "thread"]) {
        "🔩"
    } else if has(&["restri", "relaç", "junta", "acopla", "link"]) {
        "🔗"
    } else if has(&["material", "textura", "nome e exib", "aparência"]) {
        "🎨"
    } else if has(&["massa", "medidas"]) {
        "⚖"
    } else if has(&["posição", "mover", "orientação", "direção", "eixo", "eop"]) {
        "✥"
    } else if has(&["extensão", "limite", "boca", "fundo"]) {
        "↔"
    } else if has(&["padrão", "repetir", "centros"]) {
        "⁂"
    } else if has(&["texto"]) {
        "🅰"
    } else if has(&["dobra", "parede", "aba", "joggle", "estampo", "veneziana", "flangeado", "chapa", "recorte", "rasgo", "compensação", "cantos", "base (web)", "suporte"]) {
        "▱"
    } else if has(&["movimento", "comando", "simula", "explod"]) {
        "⟳"
    } else if has(&["operação", "costurar", "dividir", "fechar", "engrossar", "preencher", "malha"]) {
        "⊕"
    } else if has(&["estado", "usado por", "interfer"]) {
        "ⓘ"
    } else if has(&["tipo", "forma", "definição", "plano"]) {
        "☰"
    } else if has(&["componente", "peças", "montagem", "recurso"]) {
        "🧩"
    } else if has(&["parâmetro", "propriedades"]) {
        "ƒ"
    } else {
        "•"
    }
}

/// Ícone de uma opção liga/desliga, pelo texto.
pub(crate) fn check_icon(label: &str) -> &'static str {
    let l = label.to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    if has(&["invert", "outro lado", "reverse", "flip", "sentido"]) {
        "⇄"
    } else if has(&["suprim", "desativ"]) {
        "⊘"
    } else if has(&["simétric", "simetric", "dois lados", "meio"]) {
        "⇋"
    } else if has(&["fix", "travar", "trava"]) {
        "📌"
    } else if has(&["mostrar", "exibir", "visível", "visivel", "ocult", "visualiza"]) {
        "👁"
    } else if has(&["raio", "arredond"]) {
        "◜"
    } else if has(&["alívio", "alivio", "recuo", "folga", "relimit"]) {
        "⇹"
    } else if has(&["meia-esquadria", "esquadria", "canto"]) {
        "◳"
    } else if has(&["cor", "aparência", "textura"]) {
        "🎨"
    } else if has(&["relacional", "link", "associ"]) {
        "🔗"
    } else if has(&["fino", "parede", "espessura", "casca"]) {
        "▭"
    } else if has(&["automátic", "automatic", "auto"]) {
        "⟳"
    } else if has(&["próprio", "proprio", "personaliz"]) {
        "✎"
    } else if has(&["planific", "dobr"]) {
        "▱"
    } else {
        "◦"
    }
}

/// Caixa de marcar com ícone pelo significado (texto traduzido).
pub(crate) fn check(ui: &mut egui::Ui, v: &mut bool, label: &str) -> egui::Response {
    let label = tr(label);
    // o texto já começa com um símbolo: não repete
    if label.chars().next().is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace() && c != '(') {
        return ui.checkbox(v, label);
    }
    ui.checkbox(v, format!("{} {label}", check_icon(label)))
}

/// Dica das caixas de expressão.
pub(crate) const EXPR_HINT: &str = "Aceita número (vírgula ou ponto), parâmetro ou expressão: L/2 + 5, 2*pi*r, sqrt(a; b). Roda do mouse soma/subtrai (Shift ×10, Ctrl ×0,1).";

/// Campo de expressão: mostra o valor avaliado ou o erro.
pub(crate) fn expr(ui: &mut egui::Ui, label: &str, s: &mut String, vars: &Vars, unit: &str) {
    let label = tr(label);
    ui.horizontal(|ui| {
        let icon = param_icon(label);
        let lr = ui.add_sized([118., 18.], egui::Label::new(RichText::new(format!("{icon} {label}"))).truncate().sense(egui::Sense::hover()));
        lr.on_hover_ui(|ui| {
            ui.label(RichText::new(format!("{icon} {label}")).strong());
            if let Ok(v) = fenkra_core::expr::eval_str(s, vars) {
                ui.label(format!("= {} {unit}", fmt(v)));
            }
            ui.label(RichText::new(tr(EXPR_HINT)).weak().small());
        });
        let r = fenkra_core::expr::eval_str(s, vars);
        let bad = r.is_err();
        let mut te = egui::TextEdit::singleline(s).desired_width(110.);
        if bad {
            te = te.text_color(crate::theme::st_err());
        }
        let resp = ui.add(te);
        // roda do mouse sobre o campo soma/subtrai (Shift ×10, Ctrl ×0,1)
        crate::ui_ext::wheel_text(ui, &resp, s);
        let resp = if bad { resp } else { resp.on_hover_text(tr(EXPR_HINT)) };
        match r {
            Ok(v) if s.trim().replace(',', ".").parse::<f64>().is_err() => {
                ui.label(RichText::new(format!("= {} {unit}", fmt(v))).weak());
            }
            Ok(_) => {
                ui.label(RichText::new(unit).weak());
            }
            Err(e) => {
                resp.on_hover_text(e.to_string());
                ui.label(RichText::new(tr("⚠")).color(crate::theme::st_err()));
            }
        }
    });
}

/// Abas do painel em edição: cada `section` vira uma aba (as abas vêm da lista do quadro
/// anterior). A aba ativa aparece num cartão; as outras são desenhadas fora de vista (sem pintura
/// nem clique) e o cursor volta, então o painel não muda de código para ganhar abas.
struct TabRun {
    key: egui::Id,
    ui_id: egui::Id,
    prev: Vec<String>,
    titles: Vec<String>,
    active: usize,
    tabs: bool,
    /// seção atual: escondida (y de início, recorte salvo) ou visível (cartão)
    hidden: Option<(f32, egui::Rect)>,
    card: Option<(egui::layers::ShapeIdx, f32)>,
}

thread_local! {
    static TABS: std::cell::RefCell<Option<TabRun>> = const { std::cell::RefCell::new(None) };
}

/// Começa o painel com abas (`key` identifica o tipo de comando: a aba escolhida é lembrada).
pub(crate) fn tabs_begin(ui: &mut egui::Ui, key: impl std::hash::Hash) {
    let h = {
        use std::hash::Hasher;
        let mut st = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut st);
        st.finish()
    };
    let key = egui::Id::unique(("abas_painel", h));
    let prev: Vec<String> = ui.ctx().data(|d| d.get_temp::<Vec<String>>(key)).unwrap_or_default();
    let active: usize = ui.ctx().data(|d| d.get_temp::<usize>(key.with("ativa"))).unwrap_or(0);
    TABS.with(|t| *t.borrow_mut() = Some(TabRun { key, ui_id: ui.scope_id(), prev, titles: vec![], active, tabs: false, hidden: None, card: None }));
}

/// Fecha a seção atual: volta o cursor (escondida) ou pinta o cartão por baixo (visível).
fn tab_close(ui: &mut egui::Ui, run: &mut TabRun) {
    if let Some((y0, clip)) = run.hidden.take() {
        ui.set_clip_rect(clip);
        let dy = ui.cursor().top() - y0;
        if dy > 0. {
            ui.add_space(-dy);
        }
    }
    if let Some((slot, top)) = run.card.take() {
        let r = egui::Rect::from_min_max(egui::pos2(ui.max_rect().left() - 4., top), egui::pos2(ui.max_rect().right() + 4., ui.cursor().top() + 4.));
        let fill = if ui.visuals().dark_mode { Color32::from_rgba_unmultiplied(255, 255, 255, 10) } else { Color32::from_rgba_unmultiplied(0, 0, 0, 10) };
        ui.painter().set(slot, egui::Shape::rect_filled(r, 10., fill));
        ui.add_space(10.);
    }
}

/// Termina o painel com abas (guarda a lista para o próximo quadro).
pub(crate) fn tabs_end(ui: &mut egui::Ui) {
    let Some(mut run) = TABS.with(|t| t.borrow_mut().take()) else { return };
    tab_close(ui, &mut run);
    ui.ctx().data_mut(|d| d.insert_temp(run.key, run.titles.clone()));
}

/// Barra de abas: poucas abas numa linha; muitas viram uma grade de botões iguais (2 ou 3 por
/// linha, conforme a largura), com ícone, para o painel não virar uma fita comprida.
fn tab_bar(ui: &mut egui::Ui, run: &mut TabRun) {
    ui.add_space(4.);
    let accent = crate::theme::accent(ui.visuals());
    let n = run.prev.len();
    let w = ui.available_width();
    let cols = if n <= 3 { n.max(1) } else if w < 250. { 2 } else { 3 };
    let gap = 4.;
    let cell_w = ((w - gap * (cols as f32 - 1.)) / cols as f32).floor();
    let t = ui.input(|i| i.time) as f32;
    let rows: Vec<(usize, Vec<String>)> = run.prev.chunks(cols).enumerate().map(|(r, c)| (r * cols, c.to_vec())).collect();
    for (first, row) in rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            for (k, title) in row.iter().enumerate() {
                let i = first + k;
                let on = i == run.active;
                let (r, resp) = ui.allocate_exact_size(egui::vec2(cell_w, 26.), egui::Sense::click());
                let hot = ui.ctx().animate_bool(egui::Id::unique(("aba_hot", run.key, i)), resp.hovered() || on);
                let v = ui.visuals();
                let fill = if on { accent } else { v.faint_bg_color.lerp_to_gamma(accent, 0.25 * hot) };
                ui.painter().rect_filled(r, 8., fill);
                if !on {
                    ui.painter().rect_stroke(r, 8., egui::Stroke::new(1., v.widgets.inactive.bg_stroke.color.gamma_multiply(0.7)), egui::StrokeKind::Inside);
                } else {
                    // brilho suave correndo na aba ativa
                    let x = r.left() + ((t * 0.35).fract()) * (r.width() + 40.) - 20.;
                    let sh = egui::Rect::from_min_max(egui::pos2(x.max(r.left() + 4.), r.top() + 2.), egui::pos2((x + 20.).min(r.right() - 4.), r.bottom() - 2.));
                    if sh.width() > 0. {
                        ui.painter().rect_filled(sh, 6., Color32::from_white_alpha(14));
                    }
                }
                let col = if on { Color32::WHITE } else { v.text_color() };
                let mut job = egui::text::LayoutJob::simple_singleline(format!("{} {}", section_icon(title), title), egui::FontId::proportional(12.), col);
                job.wrap = egui::text::TextWrapping::truncate_at_width(r.width() - 10.);
                let g = ui.painter().layout_job(job);
                ui.painter().galley(r.center() - g.size() / 2., g, col);
                let resp = resp.on_hover_text(title.as_str());
                if resp.clicked() {
                    run.active = i;
                    ui.ctx().data_mut(|d| d.insert_temp(run.key.with("ativa"), i));
                }
            }
        });
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    ui.add_space(8.);
}

pub(crate) fn section(ui: &mut egui::Ui, title: &str) {
    let title = tr(title);
    let handled = TABS.with(|t| {
        let mut b = t.borrow_mut();
        let Some(run) = b.as_mut() else { return false };
        // só no corpo principal do painel (seções dentro de grupos ficam como título simples)
        if ui.scope_id() != run.ui_id {
            return false;
        }
        tab_close(ui, run);
        if run.titles.is_empty() {
            // primeira seção: abas se o quadro anterior tinha 2 ou mais
            run.tabs = run.prev.len() >= 2;
            if run.active >= run.prev.len() {
                run.active = 0;
            }
            if run.tabs {
                tab_bar(ui, run);
            }
        }
        let idx = run.titles.len();
        run.titles.push(title.to_string());
        let visible = !run.tabs || idx == run.active;
        if visible {
            let top = ui.cursor().top();
            let slot = ui.painter().add(egui::Shape::Noop);
            run.card = Some((slot, top));
            ui.add_space(6.);
            ui.label(RichText::new(format!("{}  {}", section_icon(title), title.to_uppercase())).size(11.).strong().color(ui.visuals().weak_text_color()));
            ui.add_space(2.);
        } else {
            run.hidden = Some((ui.cursor().top(), ui.clip_rect()));
            ui.set_clip_rect(egui::Rect::NOTHING);
        }
        true
    });
    if !handled {
        ui.add_space(6.);
        ui.label(RichText::new(format!("{} {title}", section_icon(title))).strong());
    }
}

/// Selo de estado (como "ATIVO" nos menus do sistema).
pub(crate) fn status_chip(ui: &mut egui::Ui, txt: &str, col: Color32) {
    let col = crate::theme::readable(col, ui.visuals());
    let g = ui.painter().layout_no_wrap(txt.to_string(), egui::FontId::proportional(11.), col);
    let (r, _) = ui.allocate_exact_size(g.size() + egui::vec2(16., 8.), egui::Sense::hover());
    ui.painter().rect_filled(r, 7., col.gamma_multiply(0.18));
    ui.painter().galley(r.min + egui::vec2(8., 4.), g, col);
}

pub(crate) fn plane_combo(ui: &mut egui::Ui, id: &str, doc: &Document, p: &mut PlaneRef, exclude: Option<Id>) {
    // sem o próprio elemento e sem quem depende dele (evita ciclos)
    let deps = exclude.map(|x| doc.dependents(x)).unwrap_or_default();
    egui::ComboBox::from_id_salt(id).selected_text(plane_ref_name(doc, p)).width(150.).show_ui(ui, |ui| {
        for o in [OriginPlane::XY, OriginPlane::XZ, OriginPlane::YZ] {
            ui.selectable_value(p, PlaneRef::Origin(o), format!("Origem · {}", plane_name(o)));
        }
        for r in doc.references.iter().filter(|r| Some(r.id) != exclude && r.kind.class() == RefClass::Plane && !deps.contains(&r.id)) {
            ui.selectable_value(p, PlaneRef::Datum(r.id), &r.name);
        }
    });
}

fn sketch_combo(ui: &mut egui::Ui, id: &str, doc: &Document, before: usize, s: &mut Id) {
    let text = profile_text(doc, *s);
    egui::ComboBox::from_id_salt(id).selected_text(text).width(200.).show_ui(ui, |ui| profile_options(ui, doc, before, s));
}

/// Nome do perfil escolhido (sketch ou curva do wireframe), em laranja quando falta escolher.
pub fn profile_text(doc: &Document, s: Id) -> RichText {
    let name = doc
        .feature(s)
        .map(|f| f.name.clone())
        .or_else(|| doc.reference(s).map(|r| format!("〰 {}", r.name)))
        .or_else(|| crate::slots::label(s))
        .unwrap_or_else(|| format!("⚠ perdido #{}", s.0));
    if crate::slots::is_unset(s) {
        RichText::new(name).color(Color32::from_rgb(255, 170, 60))
    } else {
        RichText::new(name)
    }
}

/// Opções de perfil: sketches antes do comando e os livres (pastas de geometria), mais as curvas
/// do wireframe (fechadas e planas viram perfil; o plano sai da própria curva).
pub fn profile_options(ui: &mut egui::Ui, doc: &Document, before: usize, s: &mut Id) {
    for (i, f) in doc.features.iter().enumerate().filter(|(_, f)| matches!(f.kind, FeatureKind::Sketch(_))) {
        if i < before || doc.is_free(f) {
            ui.selectable_value(s, f.id, &f.name);
        }
    }
    let curves: Vec<_> = doc.references.iter().filter(|r| r.kind.class() == RefClass::Curve).collect();
    if !curves.is_empty() {
        ui.separator();
        ui.label(RichText::new(tr("Curvas do wireframe")).weak().small());
        for r in curves {
            ui.selectable_value(s, r.id, format!("〰 {}", r.name));
        }
    }
}

/// Linha de opções com ícone + nome (uma variação = um ícone). Devolve o índice clicado.
pub fn icon_choice(ui: &mut egui::Ui, opts: &[(&str, &str, &str, bool)]) -> Option<usize> {
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.;
        for (i, (icon, name, tip, sel)) in opts.iter().enumerate() {
            if crate::icons::tool_button(ui, icon, name, 26., true, *sel, tip, "").clicked() {
                hit = Some(i);
            }
        }
    });
    hit
}

fn op_combo(ui: &mut egui::Ui, _id: &str, op: &mut BoolOp) {
    ui.label(RichText::new(tr("Operação")).weak());
    let o = *op;
    let opts = [
        ("op_adicionar", "Adicionar", "Une ao corpo", o == BoolOp::Add),
        ("op_remover", "Remover", "Subtrai do corpo", o == BoolOp::Cut),
        ("op_intersecao", "Interseção", "Mantém só a parte comum", o == BoolOp::Intersect),
    ];
    if let Some(i) = icon_choice(ui, &opts) {
        *op = [BoolOp::Add, BoolOp::Cut, BoolOp::Intersect][i];
    }
}

pub(crate) fn extent_ui(ui: &mut egui::Ui, _id: &str, e: &mut Extent, vars: &Vars, allow: &[&str], doc: &Document, seed: &crate::geo_ui::Seed) {
    let cur = match e {
        Extent::Blind(_) => "Cego",
        Extent::Symmetric(_) => "Plano médio",
        Extent::TwoSides(..) => "Dois sentidos",
        Extent::ThroughAll => "Passante",
        Extent::UpToLast => "Até o último",
        Extent::UpTo(_) => "Até plano",
        Extent::UpToNext => "Até o próximo",
        Extent::UpToSurface(_) => "Até a superfície",
    };
    let keep = match e {
        Extent::Blind(d) | Extent::Symmetric(d) | Extent::TwoSides(d, _) => d.clone(),
        _ => "20".into(),
    };
    ui.label(RichText::new(tr("Término")).weak());
    let info = |n: &str| match n {
        "Cego" => ("ext_cego", "Distância fixa a partir do plano do sketch"),
        "Plano médio" => ("ext_simetrico", "Metade para cada lado do plano do sketch"),
        "Dois sentidos" => ("ext_dois_sentidos", "Distâncias diferentes para cada lado"),
        "Passante" => ("ext_passante", "Atravessa todo o corpo"),
        "Até o último" => ("ext_ate_ultimo", "Vai até a última parede do corpo nessa direção"),
        "Até o próximo" => ("ext_ate_face", "Para na primeira parede do corpo nessa direção"),
        "Até a superfície" => ("ext_ate_superficie", "Termina numa superfície ou face (estendida se for menor que o perfil)"),
        _ => ("ext_ate_plano", "Termina exatamente num plano (inclusive inclinado)"),
    };
    let opts: Vec<(&str, &str, &str, bool)> = allow.iter().map(|n| (info(n).0, *n, info(n).1, cur == *n)).collect();
    if let Some(i) = icon_choice(ui, &opts) {
        let n = allow[i];
        if n != cur {
            *e = match n {
                "Cego" => Extent::Blind(keep.clone()),
                "Plano médio" => Extent::Symmetric(keep.clone()),
                "Dois sentidos" => Extent::TwoSides(keep.clone(), keep.clone()),
                "Passante" => Extent::ThroughAll,
                "Até o próximo" => Extent::UpToNext,
                "Até o último" => Extent::UpToLast,
                "Até a superfície" => Extent::UpToSurface(seed.surface.unwrap_or_else(|| {
                    doc.references
                        .iter()
                        .find(|r| r.kind.class() == RefClass::Surface)
                        .map(|r| SurfRef::Ref(r.id))
                        .unwrap_or(SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)))
                })),
                _ => Extent::UpTo(doc.references.first().map(|r| PlaneRef::Datum(r.id)).unwrap_or(PlaneRef::Origin(OriginPlane::XY))),
            };
        }
    }
    match e {
        Extent::Blind(d) => expr(ui, "Distância", d, vars, "mm"),
        Extent::Symmetric(d) => expr(ui, "Total", d, vars, "mm"),
        Extent::TwoSides(a, b) => {
            expr(ui, "Sentido 1", a, vars, "mm");
            expr(ui, "Sentido 2", b, vars, "mm");
        }
        Extent::ThroughAll => {
            ui.label(RichText::new(tr("atravessa todo o corpo de destino")).weak().small());
        }
        Extent::UpToLast => {
            ui.label(RichText::new(tr("vai até a parede mais distante do corpo nessa direção")).weak().small());
        }
        Extent::UpToNext => {
            ui.label(RichText::new(tr("para na primeira parede do corpo encontrada")).weak().small());
        }
        Extent::UpToSurface(s) => {
            crate::geo_ui::surf_slot(ui, "ate_sup", "Até a superfície", doc, Id(u64::MAX), s, seed);
            ui.label(RichText::new(tr("Selecione uma face na peça ou uma superfície da pasta de geometria e clique em ⊙.")).weak().small());
        }
        Extent::UpTo(p) => {
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Até o plano")));
                plane_combo(ui, "ate_plano", doc, p, None);
            });
            ui.label(RichText::new(tr("Crie planos inclinados em Referências › Plano por ângulo.")).weak().small());
        }
    }
}

fn target_combo(ui: &mut egui::Ui, doc: &Document, t: &mut Id) {
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Coletor")));
        let name = doc.collectors.iter().find(|c| c.id == *t).map(|c| c.name.clone()).unwrap_or("?".into());
        egui::ComboBox::from_id_salt("alvo").selected_text(name).show_ui(ui, |ui| {
            for c in &doc.collectors {
                ui.selectable_value(t, c.id, &c.name);
            }
        });
    });
}

/// Lista "onde é usado" (RF-LNK-022).
fn used_by(ui: &mut egui::Ui, doc: &Document, id: Id, sel: &mut Option<Sel>) {
    let deps = doc.dependents(id);
    if deps.is_empty() {
        return;
    }
    section(ui, &format!("Usado por ({})", deps.len()));
    for d in deps {
        if ui.link(doc.name_of(d)).clicked() {
            *sel = Some(if doc.reference(d).is_some() { Sel::Ref(d) } else { Sel::Feature(d) });
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let vars = app.vars();
    let mut new_sel = None;
    let mut delete = None;
    let status = match app.sel {
        Sel::Feature(id) => app.status_of(id).cloned(),
        _ => None,
    };
    let sel = app.sel;
    let doc_ro = app.open.doc.clone();
    let frozen = app.open.frozen.clone();
    let bodies: Vec<(Id, f64, f64)> = app.result.as_ref().map(|r| r.bodies.iter().map(|b| (b.collector, b.mass.volume, b.mass.area)).collect()).unwrap_or_default();
    let picked = app.picked_edges();
    let picked_faces = app.picked_faces();
    let session = app.session.as_ref().map(|s| (s.id, s.is_new));
    let seed = app.seed(false);
    let ref_err = match sel {
        Sel::Ref(id) => match app.status_of(id) {
            Some(Status::Error(e)) => Some(e.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut sess_act: Option<u8> = None; // 0 = concluir, 1 = cancelar, 2 = abrir
    let tex_names: Vec<String> = app.open.doc.textures.iter().map(|t| t.name.clone()).collect();
    let doc = &mut app.open.doc;

    if let Some(rev) = &frozen {
        egui::Frame::NONE.fill(Color32::from_rgb(70, 55, 25)).inner_margin(6.).corner_radius(4.).show(ui, |ui| {
            ui.label(RichText::new(format!("🔒 Revisão {rev} congelada — somente leitura")).color(Color32::from_rgb(255, 210, 120)));
        });
    }

    match sel {
        Sel::None => {
            if app.settings.bench == crate::settings::Bench::Chapa {
                ui.heading(tr("Chapa metálica"));
                ui.label(RichText::new(tr("▭ Parâmetros (espessura, raio, fator K) → sketch → Parede base → abas, bainhas, joggles, estampos e furos flangeados nas arestas e faces. Dobrar/Desdobrar alterna com o planificado.")).weak());
            } else if app.settings.bench == crate::settings::Bench::Superficies {
                ui.heading(tr("Superfícies"));
                ui.label(RichText::new(tr("Wireframe (Referências) → superfícies → operações → Volume para virar sólido. As superfícies ficam na pasta de geometria e servem de entrada para os comandos de sólido.")).weak());
            } else {
                ui.heading(tr("Modelagem Sólida"));
                ui.label(RichText::new(tr("Escolha um comando na barra ou clique num plano/face na viewport.")).weak());
            }
            ui.add_space(6.);
            ui.label(RichText::new(tr("Atalhos")).strong());
            ui.label(RichText::new(tr("Espaço: vistas · F: ajustar · N: alinhar ao plano · H: ocultar · Esc: limpar seleção")).weak().small());
        }

        Sel::Part => {
            ui.heading(tr("Peça"));
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Nome")));
                ui.text_edit_singleline(&mut doc.name);
            });
            section(ui, "Propriedades");
            let p = &mut doc.props;
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("PN")));
                ui.text_edit_singleline(&mut p.pn);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Descrição")));
                ui.text_edit_singleline(&mut p.description);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Material")));
                egui::ComboBox::from_id_salt("mat").selected_text(p.material.clone()).show_ui(ui, |ui| {
                    for (m, _) in MATERIALS {
                        ui.selectable_value(&mut p.material, m.to_string(), m);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Tipo")));
                egui::ComboBox::from_id_salt("tipo").selected_text(p.part_type.label()).show_ui(ui, |ui| {
                    for t in PartType::ALL {
                        ui.selectable_value(&mut p.part_type, t, t.label());
                    }
                });
            });
            section(ui, "Projeto estrutural");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Barra máx.")));
                let r = ui.add(egui::TextEdit::singleline(&mut p.stock_length).desired_width(80.).hint_text(tr("sem limite")));
                crate::ui_ext::wheel_text(ui, &r, &mut p.stock_length);
                ui.label("mm");
            })
            .response
            .on_hover_text(tr("Comprimento da matéria-prima (ex.: 6000). Membros maiores avisam que precisam de emenda, também na lista de corte."));
            section(ui, "Propriedades personalizadas");
            let mut remove = None;
            for (i, (k, v)) in p.custom.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(k).desired_width(96.));
                    ui.add(egui::TextEdit::singleline(v).desired_width(140.));
                    if ui.small_button(tr("✖")).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                p.custom.remove(i);
            }
            if ui.button(tr("➕ Propriedade")).clicked() {
                p.custom.push((format!("Campo{}", p.custom.len() + 1), String::new()));
            }
            section(ui, "Propriedades de massa");
            let dens = MATERIALS.iter().find(|(m, _)| *m == p.material).map(|(_, d)| *d).unwrap_or(7.85);
            for (cid, vol, area) in &bodies {
                let name = doc.collectors.iter().find(|c| c.id == *cid).map(|c| c.name.clone()).unwrap_or_default();
                ui.label(format!("{name}: {}", crate::app::loc(format!("{:.1} cm³ · {:.1} cm² · {:.3} kg", vol / 1000., area / 100., vol * dens / 1e6))));
            }
            if bodies.is_empty() {
                ui.label(RichText::new(tr("sem corpos")).weak());
            }
            section(ui, "Aparência da peça");
            ui.label(RichText::new(tr("Sobrepõe a cor dos coletores; comandos com aparência própria têm prioridade.")).weak().small());
            let tex: Vec<String> = doc.textures.iter().map(|t| t.name.clone()).collect();
            appearance_opt(ui, "ap_peca", &mut doc.appearance, &tex);
            section(ui, "Texturas da peça");
            let mut del = None;
            for (i, t) in doc.textures.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&t.name);
                    { let __r = ui.add(crate::ui_ext::dv(&mut t.scale).range(0.1..=10000.).speed(1.).suffix(" mm")); crate::ui_ext::wheel(ui, &__r, &mut t.scale); __r }
                        .on_hover_text(tr("tamanho de uma repetição da imagem"));
                    if ui.small_button("✖").clicked() {
                        del = Some(i);
                    }
                });
            }
            if let Some(i) = del {
                doc.textures.remove(i);
            }
            if doc.textures.len() < 8 && ui.button(tr("🖼 Adicionar imagem…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Imagem", &["png", "jpg", "jpeg"]).pick_file() {
                    let name = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                    doc.textures.push(TextureRef { name, path: p.to_string_lossy().to_string(), scale: 50. });
                }
            }
            ui.label(RichText::new(tr("Projetada nos três planos (triplanar) e multiplicada pela cor: use branco para a imagem pura.")).weak().small());
        }

        Sel::Origin(o) => {
            ui.heading(format!("Plano {} da Origem", plane_name(o)));
            ui.label(RichText::new(tr("Os elementos da Origem são fixos: não podem ser apagados nem movidos.")).weak());
            ui.add_space(6.);
            ui.label(tr("Use os botões ✏ Sketch ou ▱ Plano para criar a partir deste plano."));
            used_by_origin(ui, &doc_ro, o, &mut new_sel);
        }

        Sel::Ref(id) => {
            let names: Vec<String> = doc.params.iter().map(|p| p.name.clone()).chain(doc.features.iter().map(|f| f.name.clone())).collect();
            let Some(r) = doc.references.iter_mut().find(|r| r.id == id) else {
                ui.label(tr("Referência não existe mais."));
                return;
            };
            command_bar(ui, session.map(|s| s.0) == Some(id), &mut sess_act);
            tabs_begin(ui, ("elemento", format!("{:?}", crate::geo_ui::GeoKind::of(&r.kind))));
            section(ui, "Definição");
            crate::geo_ui::panel(ui, &doc_ro, r, &vars, &seed, ref_err.as_deref());
            section(ui, "Nome e exibição");
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Nome (variável)")));
                let mut n = r.name.clone();
                if ui.text_edit_singleline(&mut n).changed() && !n.is_empty() && !names.contains(&n) {
                    r.name = n;
                }
            });
            ui.label(RichText::new(tr("O nome é a variável referenciável; os comandos apontam para o elemento, então renomear não quebra nada.")).weak().small());
            crate::editor::check(ui, &mut r.visible, "visível");
            if r.kind.class() != RefClass::Surface && !matches!(r.kind, RefKind::Linked { .. }) {
                crate::editor::check(ui, &mut r.published, "📤 publicar (outras peças podem usar por link)");
            }
            ui.add_space(4.);
            ui.label(RichText::new(tr("Trocar uma entrada redireciona a variável: tudo que usa este elemento se atualiza.")).weak().small());
            section(ui, "Relações");
            used_by(ui, &doc_ro, id, &mut new_sel);
            ui.add_space(8.);
            if ui.button(tr("🗑 Apagar…")).clicked() {
                delete = Some(id);
            }
            tabs_end(ui);
        }

        Sel::Params => params_table(ui, doc, &vars, &mut app_toast_sink()),
        Sel::Pmi(id) => {
            crate::pmi_ui::panel(app, ui, id);
        }

        Sel::Collector(id) => {
            let active = doc.active_collector == id;
            let users: Vec<String> = doc.features.iter().filter(|f| f.target == id && !matches!(f.kind, FeatureKind::Sketch(_))).map(|f| f.name.clone()).collect();
            let Some(c) = doc.collectors.iter_mut().find(|c| c.id == id) else { return };
            ui.heading(tr("Coletor de corpos"));
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Nome")));
                ui.text_edit_singleline(&mut c.name);
            });
            let mut a = Appearance { color: c.color, finish: c.finish, texture: c.texture };
            appearance_fields(ui, "ap_col", &mut a, &tex_names);
            c.color = a.color;
            c.finish = a.finish;
            c.texture = a.texture;
            crate::editor::check(ui, &mut c.visible, "visível");
            if active {
                ui.label(RichText::new(tr("✔ coletor ativo: novos comandos atuam nele")).color(Color32::from_rgb(140, 210, 140)));
            } else if ui.button(tr("Tornar ativo")).clicked() {
                doc.active_collector = id;
            }
            section(ui, &format!("Comandos neste coletor ({})", users.len()));
            for u in users {
                ui.label(format!("• {u}"));
            }
        }

        Sel::Folder(id) => {
            let Some(f) = doc.folders.iter_mut().find(|f| f.id == id) else { return };
            ui.heading(tr("Pasta"));
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Nome")));
                ui.text_edit_singleline(&mut f.name);
            });
            ui.label(RichText::new(tr("Pastas só organizam a árvore: não mudam a ordem de execução.")).weak());
            let items: Vec<String> = doc.features.iter().filter(|g| g.folder == Some(id)).map(|g| g.name.clone()).collect();
            section(ui, &format!("Conteúdo ({})", items.len()));
            for i in items {
                ui.label(format!("• {i}"));
            }
            ui.add_space(8.);
            if ui.button(tr("Remover pasta (mantém os comandos)")).clicked() {
                doc.remove_folder(id);
                new_sel = Some(Sel::None);
            }
        }

        Sel::Feature(id) if session.map(|s| s.0) != Some(id) && !matches!(doc.feature(id).map(|f| &f.kind), Some(FeatureKind::Sketch(_))) => {
            // resumo limpo: o comando só abre para edição com duplo clique / Editar
            let Some(f) = doc.feature_mut(id) else { return };
            ui.heading(format!("{} {}", f.kind.icon(), f.name));
            ui.label(RichText::new(f.kind.type_name()).weak());
            match &status {
                Some(Status::Error(m)) => {
                    ui.label(RichText::new(format!("✖ {m}")).color(crate::theme::st_err()));
                }
                Some(Status::Warning(m)) => {
                    ui.label(RichText::new(format!("⚠ {m}")).color(crate::theme::st_warn()));
                }
                _ => {}
            }
            ui.add_space(6.);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("✏ Editar")).strong()).min_size(egui::vec2(110., 28.))).clicked() {
                    sess_act = Some(2);
                }
                let mut vis = !f.hidden;
                if crate::editor::check(ui, &mut vis, "visível").changed() {
                    f.hidden = !vis;
                }
                crate::editor::check(ui, &mut f.suppressed, "suprimido");
            });
            ui.label(RichText::new(tr("Duplo clique na árvore também abre a edição.")).weak().small());
            used_by(ui, &doc_ro, id, &mut new_sel);
        }

        Sel::Feature(id) => {
            let Some(idx) = doc.feature_index(id) else {
                ui.label(tr("Comando não existe mais."));
                return;
            };
            if session.map(|s| s.0) == Some(id) {
                command_bar(ui, true, &mut sess_act);
            }
            let names: Vec<String> = doc.params.iter().map(|p| p.name.clone()).chain(doc.references.iter().map(|r| r.name.clone())).collect();
            {
                let f = &mut doc.features[idx];
                // cabeçalho: ícone, tipo, nome e selo de estado
                egui::Frame::NONE.fill(ui.visuals().faint_bg_color).corner_radius(12.).inner_margin(10.).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(38., 38.), egui::Sense::hover());
                        let acc = crate::theme::accent(ui.visuals());
                        ui.painter().rect_filled(r, 9., acc.gamma_multiply(0.18));
                        ui.painter().rect_stroke(r, 9., egui::Stroke::new(1., acc.gamma_multiply(0.5)), egui::StrokeKind::Inside);
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, f.kind.icon(), egui::FontId::proportional(22.), acc);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(tr(f.kind.type_name())).size(17.).strong());
                            let mut n = f.name.clone();
                            if ui.add(egui::TextEdit::singleline(&mut n).desired_width(150.)).changed() && !n.is_empty() && !names.contains(&n) {
                                f.name = n;
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            let (t, c) = match &status {
                                Some(Status::Error(_)) => ("ERRO", Color32::from_rgb(255, 110, 95)),
                                Some(Status::Warning(_)) => ("AVISO", Color32::from_rgb(240, 195, 80)),
                                Some(Status::Inactive) => ("INATIVO", Color32::from_gray(150)),
                                _ => ("OK", crate::theme::st_ok()),
                            };
                            status_chip(ui, tr(t), c);
                        });
                    });
                    match &status {
                        Some(Status::Error(m)) => {
                            ui.label(RichText::new(m).color(crate::theme::st_err()).small());
                        }
                        Some(Status::Warning(m)) => {
                            ui.label(RichText::new(m).color(crate::theme::st_warn()).small());
                        }
                        Some(Status::Inactive) => {
                            ui.label(RichText::new(tr("inativo (suprimido ou depois do marcador)")).weak().small());
                        }
                        _ => {}
                    }
                });
                ui.horizontal(|ui| {
                    crate::editor::check(ui, &mut f.suppressed, "suprimido");
                    // só sketches têm olho próprio; sólidos aparecem pelo corpo (última feature)
                    if matches!(f.kind, FeatureKind::Sketch(_)) {
                        let mut vis = !f.hidden;
                        if crate::editor::check(ui, &mut vis, "visível").changed() {
                            f.hidden = !vis;
                        }
                    } else if ui.button(tr("🎨 Propriedades gráficas…")).clicked() {
                        ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("fenkra_props"), f.id));
                    }
                });
                let _ = &tex_names;
            }
            let mut target = doc.features[idx].target;
            let mut kind = doc.features[idx].kind.clone();
            tabs_begin(ui, ("comando", kind.type_name()));
            feature_ui(ui, &doc_ro, idx, id, &mut kind, &mut target, &vars, doc, &picked, &picked_faces, &seed);
            let f = &mut doc.features[idx];
            if f.kind != kind {
                f.kind = kind;
            }
            f.target = target;

            section(ui, "Relações");
            used_by(ui, &doc_ro, id, &mut new_sel);
            if matches!(doc.features[idx].kind, FeatureKind::Sketch(_)) {
                replace_all(ui, doc, id);
            }
            ui.add_space(6.);
            if ui.button(tr("🗑 Apagar…")).clicked() {
                delete = Some(id);
            }
            tabs_end(ui);
        }
    }

    if let Some(s) = new_sel {
        app.sel = s;
    }
    match (sess_act, sel) {
        (Some(0), _) if app.session.is_some() => app.conclude_session(),
        (Some(0), _) => app.sel = Sel::None,
        (Some(1), _) => app.cancel_session(),
        // excluir: comando novo some (como cancelar); existente pede confirmação dos dependentes
        (Some(3), Sel::Feature(id) | Sel::Ref(id)) => {
            if app.session.as_ref().is_some_and(|s| s.is_new && s.id == id) {
                app.cancel_session();
            } else {
                app.session = None;
                app.ask_delete(id);
            }
        }
        (Some(2), Sel::Feature(id) | Sel::Ref(id)) => app.start_session(id, false, None),
        _ => {}
    }
    if let Some(id) = ui.ctx().data_mut(|m| {
        let k = egui::Id::unique("fenkra_editar_sketch");
        let v = m.get_temp::<Id>(k);
        m.remove::<Id>(k);
        v
    }) {
        app.begin_sketch(id);
    }
    if let Some(id) = delete {
        app.ask_delete(id);
    }
}

/// Barra do comando aberto: Concluir (Enter), Cancelar (Esc, só em edição) e Excluir.
fn command_bar(ui: &mut egui::Ui, editing: bool, act: &mut Option<u8>) {
    egui::Frame::NONE.fill(ui.visuals().faint_bg_color).inner_margin(6.).corner_radius(6.).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new(tr("✔ Concluir")).strong()).min_size(egui::vec2(100., 28.))).on_hover_text(tr("Enter")).clicked() {
                *act = Some(0);
            }
            if editing && ui.add(egui::Button::new(tr("✖ Cancelar")).min_size(egui::vec2(90., 28.))).on_hover_text(tr("Esc")).clicked() {
                *act = Some(1);
            }
            if ui.add(egui::Button::new(tr("🗑 Excluir")).min_size(egui::vec2(80., 28.))).on_hover_text(tr("Del")).clicked() {
                *act = Some(3);
            }
        });
    });
}

/// Cor + acabamento + predefinições.
pub(crate) fn appearance_fields(ui: &mut egui::Ui, id: &str, a: &mut Appearance, tex: &[String]) {
    if !tex.is_empty() {
        ui.horizontal(|ui| {
            ui.add_sized([96., 18.], egui::Label::new(tr("Textura")));
            let cur = a.texture.and_then(|t| tex.get(t as usize)).cloned().unwrap_or_else(|| tr("sem textura").to_string());
            egui::ComboBox::from_id_salt((id, "tex")).selected_text(cur).show_ui(ui, |ui| {
                ui.selectable_value(&mut a.texture, None, tr("sem textura"));
                for (i, n) in tex.iter().enumerate() {
                    ui.selectable_value(&mut a.texture, Some(i as u8), n);
                }
            });
        });
    }
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Cor")));
        ui.color_edit_button_srgb(&mut a.color);
        egui::ComboBox::from_id_salt((id, "acab")).selected_text(a.finish.label()).show_ui(ui, |ui| {
            for f in Finish::ALL {
                ui.selectable_value(&mut a.finish, f, f.label());
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        for (name, p) in APPEARANCE_PRESETS {
            let c = Color32::from_rgb(p.color[0], p.color[1], p.color[2]);
            if ui.add(egui::Button::new(RichText::new(tr("■")).color(c)).small()).on_hover_text(name).clicked() {
                *a = p;
            }
        }
    });
    color_table(ui, id, &mut a.color);
    ui.horizontal(|ui| {
        if ui.small_button(tr("📋 Copiar aparência")).clicked() {
            *crate::palette::CLIP.lock().unwrap() = Some(*a);
        }
        let clip = *crate::palette::CLIP.lock().unwrap();
        if ui.add_enabled(clip.is_some(), egui::Button::new(tr("📋 Colar aparência")).small()).clicked() {
            if let Some(c) = clip {
                *a = c;
            }
        }
    });
}

/// Tabela de cores: padrões RAL com nome e as cores salvas pelo usuário (com nome).
pub(crate) fn color_table(ui: &mut egui::Ui, id: &str, color: &mut [u8; 3]) {
    let name_of = |c: [u8; 3]| {
        crate::palette::RAL.iter().find(|(_, x)| *x == c).map(|(n, _)| n.to_string())
            .or_else(|| crate::palette::USER.lock().unwrap().iter().find(|(_, x)| *x == c).map(|(n, _)| n.clone()))
    };
    ui.horizontal(|ui| {
        ui.add_sized([96., 18.], egui::Label::new(tr("Tabela")));
        egui::ComboBox::from_id_salt((id, "tabela_cores")).selected_text(name_of(*color).unwrap_or_else(|| tr("— escolher —").to_string())).width(170.).show_ui(ui, |ui| {
            let swatch = |ui: &mut egui::Ui, n: &str, c: [u8; 3], cur: &mut [u8; 3]| {
                ui.horizontal_wrapped(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(14., 14.), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2., Color32::from_rgb(c[0], c[1], c[2]));
                    if ui.selectable_label(*cur == c, n).clicked() {
                        *cur = c;
                    }
                });
            };
            let user = crate::palette::USER.lock().unwrap().clone();
            if !user.is_empty() {
                ui.label(RichText::new(tr("Minhas cores")).strong());
                for (n, c) in &user {
                    swatch(ui, n, *c, color);
                }
                ui.separator();
            }
            ui.label(RichText::new(tr("RAL clássico")).strong());
            for (n, c) in crate::palette::RAL {
                swatch(ui, n, c, color);
            }
        });
    });
    // salvar a cor atual com um nome (fica nas preferências)
    let key = egui::Id::unique(("nome_cor", id));
    let mut name: String = ui.ctx().data(|m| m.get_temp(key)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add_space(100.);
        ui.add(egui::TextEdit::singleline(&mut name).hint_text(tr("nome da cor")).desired_width(110.));
        if ui.add_enabled(!name.trim().is_empty(), egui::Button::new(tr("＋ Salvar cor")).small()).clicked() {
            let mut u = crate::palette::USER.lock().unwrap();
            u.retain(|(n, _)| n != name.trim());
            u.push((name.trim().to_string(), *color));
            *crate::palette::DIRTY.lock().unwrap() = true;
            name.clear();
        }
    });
    ui.ctx().data_mut(|m| m.insert_temp(key, name));
}

/// Aparência opcional (com botão para aplicar/remover).
pub(crate) fn appearance_opt(ui: &mut egui::Ui, id: &str, a: &mut Option<Appearance>, tex: &[String]) {
    match a {
        Some(ap) => {
            appearance_fields(ui, id, ap, tex);
            if ui.small_button(tr("remover aparência")).clicked() {
                *a = None;
            }
        }
        None => {
            if ui.button(tr("🎨 Definir aparência")).clicked() {
                *a = Some(APPEARANCE_PRESETS[0].1);
            }
        }
    }
}

/// Ninguém consome as mensagens aqui; erros de nome aparecem no próprio campo.
fn app_toast_sink() -> Vec<String> {
    vec![]
}

fn used_by_origin(ui: &mut egui::Ui, doc: &Document, o: OriginPlane, sel: &mut Option<Sel>) {
    let users: Vec<&Feature> = doc.features.iter().filter(|f| matches!(&f.kind, FeatureKind::Sketch(s) if s.plane == PlaneRef::Origin(o))).collect();
    let refs: Vec<&RefElement> = doc.references.iter().filter(|r| matches!(&r.kind, RefKind::OffsetPlane { base, .. } if *base == PlaneRef::Origin(o))).collect();
    if users.is_empty() && refs.is_empty() {
        return;
    }
    section(ui, "Usado por");
    for f in users {
        if ui.link(&f.name).clicked() {
            *sel = Some(Sel::Feature(f.id));
        }
    }
    for r in refs {
        if ui.link(&r.name).clicked() {
            *sel = Some(Sel::Ref(r.id));
        }
    }
}

/// Substituir referência em todos os usuários (RF-NUC-021).
fn replace_all(ui: &mut egui::Ui, doc: &mut Document, id: Id) {
    let users = doc.dependents(id);
    if users.is_empty() {
        return;
    }
    let others: Vec<(Id, String)> = doc.features.iter().filter(|f| f.id != id && matches!(f.kind, FeatureKind::Sketch(_))).map(|f| (f.id, f.name.clone())).collect();
    if others.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Substituir em todos os usuários por"));
        egui::ComboBox::from_id_salt("subst_all").selected_text("…").show_ui(ui, |ui| {
            for (o, n) in others {
                if ui.selectable_label(false, n).clicked() {
                    let _ = doc.replace_reference(id, o);
                }
            }
        });
    });
}

fn params_table(ui: &mut egui::Ui, doc: &mut Document, vars: &Vars, _sink: &mut Vec<String>) {
    ui.heading(tr("Parâmetros"));
    ui.label(RichText::new(tr("Expressões: + − × ÷ ^, sin cos tan sqrt abs min max round if(cond; a; b), pi. Ângulos em graus.")).weak().small());
    ui.add_space(4.);
    let mut rename: Option<(Id, String)> = None;
    let mut remove = None;
    egui::Grid::new("params").striped(true).num_columns(6).show(ui, |ui| {
        ui.label(RichText::new(tr("Nome")).strong());
        ui.label(RichText::new(tr("Expressão")).strong());
        ui.label(RichText::new(tr("Unid.")).strong());
        ui.label(RichText::new(tr("Valor")).strong());
        ui.label(RichText::new(tr("📤")).strong()).on_hover_text(tr("publicado: outras peças podem usar por link"));
        ui.label(tr(""));
        ui.end_row();
        for p in &mut doc.params {
            let mut n = p.name.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut n).desired_width(70.));
            if r.changed() {
                rename = Some((p.id, n));
            }
            ui.add(egui::TextEdit::singleline(&mut p.expr).desired_width(100.));
            egui::ComboBox::from_id_salt(("un", p.id.0)).width(40.).selected_text(match p.unit {
                Unit::Comprimento => "mm",
                Unit::Angulo => "°",
                Unit::Numero => "—",
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut p.unit, Unit::Comprimento, tr("mm"));
                ui.selectable_value(&mut p.unit, Unit::Angulo, tr("°"));
                ui.selectable_value(&mut p.unit, Unit::Numero, tr("—"));
            });
            match vars.get(&p.name) {
                Some(v) => ui.label(fmt(*v)),
                None => ui.label(RichText::new(tr("⚠")).color(crate::theme::st_err())),
            };
            ui.checkbox(&mut p.published, tr(""));
            if ui.small_button(tr("✖")).clicked() {
                remove = Some(p.id);
            }
            ui.end_row();
        }
    });
    if let Some((id, n)) = rename {
        // renomeia reescrevendo todas as expressões que usam o parâmetro
        let _ = doc.rename_param(id, &n);
    }
    if let Some(id) = remove {
        let name = doc.params.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        let mut used = false;
        let mut probe = doc.clone();
        probe.for_each_expr(|e| {
            if rename_ident(e, &name, "\u{1}") != *e {
                used = true
            }
        });
        if used {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("param_used"), name));
        } else {
            doc.params.retain(|p| p.id != id);
        }
    }
    if let Some(n) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("param_used"))) {
        ui.label(RichText::new(format!("'{n}' está em uso em expressões e não pode ser removido.")).color(crate::theme::st_warn()));
    }
    ui.add_space(6.);
    if ui.button(tr("➕ Parâmetro")).clicked() {
        let n = (1..).map(|i| format!("P{i}")).find(|n| !doc.params.iter().any(|p| &p.name == n)).unwrap();
        let _ = doc.add_param(&n, "10", Unit::Comprimento);
    }
}

#[allow(clippy::too_many_arguments)]
fn feature_ui(
    ui: &mut egui::Ui,
    doc: &Document,
    idx: usize,
    id: Id,
    kind: &mut FeatureKind,
    target: &mut Id,
    vars: &Vars,
    live: &mut Document,
    picked: &[[f64; 3]],
    picked_faces: &[[f64; 3]],
    seed: &crate::geo_ui::Seed,
) {
    if crate::feat_ui::panel(ui, doc, idx, id, kind, vars, picked_faces, seed) {
        target_combo(ui, doc, target);
        return;
    }
    match kind {
        FeatureKind::Sketch(s) => {
            section(ui, "Plano do sketch");
            plane_combo(ui, "plano_sk", doc, &mut s.plane, None);
            let d = &mut s.data;
            ui.label(format!("{} curvas · {} pontos · {} restrições", d.curves.len(), d.points.len(), d.constraints.len()));
            if ui.add(egui::Button::new(RichText::new(tr("✏ Editar sketch")).strong()).min_size(egui::vec2(160., 28.))).clicked() {
                ui.ctx().data_mut(|m| m.insert_temp(egui::Id::unique("fenkra_editar_sketch"), id));
            }
            ui.label(RichText::new(tr("Duplo clique no sketch na árvore também abre a edição.")).weak().small());
            let dims: Vec<usize> = (0..d.constraints.len()).filter(|i| d.constraints[*i].kind.is_dimension()).collect();
            if !dims.is_empty() {
                section(ui, &format!("Cotas ({})", dims.len()));
                for i in dims {
                    let c = &mut d.constraints[i];
                    let label = c.kind.label();
                    if let Some(v) = c.kind.value_mut() {
                        expr(ui, label, v, vars, "");
                    }
                }
            }
            let _ = live;
        }

        FeatureKind::Extrude(e) => {
            section(ui, "Perfil");
            sketch_combo(ui, "sk_ex", doc, idx, &mut e.sketch);
            op_combo(ui, "op_ex", &mut e.op);
            section(ui, "Extensão");
            extent_ui(ui, "ext_ex", &mut e.extent, vars, &["Cego", "Plano médio", "Dois sentidos", "Passante", "Até o próximo", "Até o último", "Até plano", "Até a superfície"], doc, seed);
            crate::editor::check(ui, &mut e.reverse, "inverter sentido");
            let mut custom = e.dir.is_some();
            if crate::editor::check(ui, &mut custom, "direção por linha, aresta ou eixo").changed() {
                e.dir = custom.then(|| seed.curves.first().map(crate::geo_ui::axis_of).unwrap_or(AxisRef::Origin(OriginAxis::Z)));
            }
            if let Some(a) = &mut e.dir {
                crate::geo_ui::axis_slot(ui, "ex_dir", "Direção", doc, Id(u64::MAX), a, seed);
            }
            crate::feat_ui::extrude_extras(ui, e, vars);
            if e.op == BoolOp::Cut {
                ui.label(RichText::new(tr("O recorte entra no material (contra a normal do plano). Use 'inverter' se necessário.")).weak().small());
            }
            target_combo(ui, doc, target);
        }

        FeatureKind::Revolve(r) => {
            section(ui, "Perfil");
            sketch_combo(ui, "sk_rv", doc, idx, &mut r.sketch);
            op_combo(ui, "op_rv", &mut r.op);
            section(ui, "Eixo");
            let axis_name = |a: &AxisRef| match a {
                AxisRef::Origin(OriginAxis::X) => "Origem · eixo X".to_string(),
                AxisRef::Origin(OriginAxis::Y) => "Origem · eixo Y".to_string(),
                AxisRef::Origin(OriginAxis::Z) => "Origem · eixo Z".to_string(),
                AxisRef::SketchLine { sketch, entity } => format!("{} · linha #{}", doc.name_of(*sketch), entity.0),
                AxisRef::Ref(id) => doc.name_of(*id),
                AxisRef::Fixed { .. } => "Aresta (direção fixa)".to_string(),
            };
            egui::ComboBox::from_id_salt("eixo").selected_text(axis_name(&r.axis)).width(200.).show_ui(ui, |ui| {
                for a in [OriginAxis::X, OriginAxis::Y, OriginAxis::Z] {
                    ui.selectable_value(&mut r.axis, AxisRef::Origin(a), axis_name(&AxisRef::Origin(a)));
                }
                for f in &doc.features[..idx] {
                    if let FeatureKind::Sketch(s) = &f.kind {
                        for e in s.data.curves.iter().filter(|e| matches!(e.geom, fenkra_core::sketch::Curve::Line { .. })) {
                            let a = AxisRef::SketchLine { sketch: f.id, entity: e.id };
                            ui.selectable_value(&mut r.axis, a, axis_name(&a));
                        }
                    }
                }
                for g in doc.references.iter().filter(|g| g.kind.class() == RefClass::Curve) {
                    ui.selectable_value(&mut r.axis, AxisRef::Ref(g.id), &g.name);
                }
            });
            expr(ui, "Ângulo", &mut r.angle, vars, "°");
            crate::editor::check(ui, &mut r.symmetric, "simétrico ao plano do sketch");
            crate::feat_ui::revolve_extras(ui, r, vars);
            target_combo(ui, doc, target);
        }

        FeatureKind::Hole(h) => hole_ui(ui, doc, idx, h, vars, target, seed),

        FeatureKind::Fillet(o) | FeatureKind::Chamfer(o) => {
            let fillet = matches!(kind_tag(id, doc), Some(true));
            // pontos antes da edição: as chaves v2 seguem os pontos que continuam na lista
            let before = match &o.edges {
                EdgeSel::Edges(m) => m.clone(),
                _ => vec![],
            };
            let lbl = if fillet { "Raio" } else { "Distância" };
            crate::feat_ui::edge_mode_ui(ui, o, vars, fillet, picked_faces);
            if matches!(o.mode, EdgeMode::FaceFace(..) | EdgeMode::FullRound { .. }) {
                target_combo(ui, doc, target);
                return;
            }
            expr(ui, lbl, &mut o.value, vars, "mm");
            ui.label(RichText::new(tr("Arestas")).weak());
            let all = matches!(o.edges, EdgeSel::AllOfBody | EdgeSel::CreatedBy(_));
            let opts = [
                ("arestas_selecionar", "Selecionadas", "Só as arestas escolhidas na viewport", !all),
                ("arestas_todas", "Todas", "Coleta todas as arestas do corpo", all),
            ];
            match icon_choice(ui, &opts) {
                Some(0) if all => o.edges = EdgeSel::Edges(picked.to_vec()),
                Some(1) => o.edges = EdgeSel::AllOfBody,
                _ => {}
            }
            if let EdgeSel::Edges(m) = &mut o.edges {
                // sacola: a lista das arestas (cada clique na aresta soma/tira)
                crate::geo_ui::bag(
                    ui,
                    &format!("arestas{}", id.0),
                    "Arestas",
                    m,
                    picked,
                    0,
                    |p| format!("{}", crate::app::p3(p, 0)),
                    |ui, _, p| {
                        ui.label(format!("{} {}", tr("aresta em"), crate::app::p3(p, 1)));
                    },
                    || [0.; 3],
                );
                ui.horizontal(|ui| {
                    if ui.add_enabled(!picked.is_empty(), egui::Button::new(format!("Usar seleção atual ({})", picked.len()))).clicked() {
                        *m = picked.to_vec();
                    }
                    if !picked.is_empty() && ui.button(tr("Somar à lista")).clicked() {
                        m.extend_from_slice(picked);
                    }
                    if !m.is_empty() && ui.button(tr("Limpar")).clicked() {
                        m.clear();
                    }
                });
                ui.label(RichText::new(tr("Clique nas arestas na viewport (Shift soma) e use 'Usar seleção atual'.")).weak().small());
                ui.label(RichText::new(tr("Propagação")).weak());
                let opts = [
                    ("prop_tangente", "Tangente", "Segue todas as arestas tangentes às escolhidas", o.prop == EdgeProp::Tangente),
                    ("prop_minima", "Mínima", "Só as arestas escolhidas: nas pontas o filete termina reto (plano normal à aresta)", o.prop == EdgeProp::Minima),
                ];
                match icon_choice(ui, &opts) {
                    Some(0) => o.prop = EdgeProp::Tangente,
                    Some(1) => o.prop = EdgeProp::Minima,
                    _ => {}
                }
            }
            // ponto novo fica sem chave (o recompute a calcula); "Todas" não usa chaves
            match &o.edges {
                EdgeSel::Edges(m) => o.keys = carry_keys(&before, &o.keys, m),
                _ => o.keys.clear(),
            }
            target_combo(ui, doc, target);
        }
        // demais comandos: painel em feat_ui
        _ => {}
    }
}


/// true = filete, false = chanfro
fn kind_tag(id: Id, doc: &Document) -> Option<bool> {
    doc.feature(id).map(|f| matches!(f.kind, FeatureKind::Fillet(_)))
}

/// Furo inteligente (RF-SOL-007/008): escolher norma e uso preenche os diâmetros.
fn hole_ui(ui: &mut egui::Ui, doc: &Document, idx: usize, h: &mut Hole, vars: &Vars, target: &mut Id, seed: &crate::geo_ui::Seed) {
    section(ui, "Centros");
    let by_pts = !h.points.is_empty();
    let opts = [
        ("sketch", "Sketch", "Os pontos de um sketch são os centros", !by_pts),
        ("ponto_superficie", "Pontos 3D", "Pontos sobre a peça (vários furos de uma vez), normais à superfície", by_pts),
    ];
    match icon_choice(ui, &opts) {
        Some(0) => h.points.clear(),
        Some(1) if !by_pts => h.points = if seed.points.is_empty() { vec![PointRef::Origin] } else { seed.points.clone() },
        _ => {}
    }
    if h.points.is_empty() {
        sketch_combo(ui, "sk_furo", doc, idx, &mut h.sketch);
        ui.label(RichText::new(tr("Os pontos do sketch são os centros; o furo entra contra a normal do plano.")).weak().small());
    } else {
        crate::geo_ui::points_list(ui, "furo_pts", doc, Id(u64::MAX), &mut h.points, seed, 1);
        ui.horizontal(|ui| {
            ui.menu_button(tr("+ pontos de um sketch"), |ui| {
                for f in doc.features.iter() {
                    let FeatureKind::Sketch(sk) = &f.kind else { continue };
                    let pts: Vec<Id> = sk.data.curves.iter().filter(|c| !c.construction).filter_map(|c| if let fenkra_core::sketch::Curve::Point { p } = c.geom { Some(p) } else { None }).collect();
                    if !pts.is_empty() && ui.button(format!("{} ({})", f.name, pts.len())).clicked() {
                        h.points.retain(|p| !matches!(p, PointRef::Origin));
                        h.points.extend(pts.into_iter().map(|point| PointRef::SketchPoint { sketch: f.id, point }));
                        ui.close();
                    }
                }
            });
            ui.menu_button(tr("+ pontos de uma pasta"), |ui| {
                for fo in doc.folders.iter().filter(|f| f.kind == FolderKind::Geometria) {
                    let pts: Vec<Id> = doc.references.iter().filter(|r| r.folder == Some(fo.id) && r.kind.class() == RefClass::Point).map(|r| r.id).collect();
                    if !pts.is_empty() && ui.button(format!("{} ({})", fo.name, pts.len())).clicked() {
                        h.points.retain(|p| !matches!(p, PointRef::Origin));
                        h.points.extend(pts.into_iter().map(PointRef::Ref));
                        ui.close();
                    }
                }
            });
        });
        ui.label(RichText::new(tr("Cada ponto vira um furo. Pontos fora da peça são projetados na superfície (na direção do furo, ou na normal da face mais perto); se não encontrarem a peça, o comando avisa.")).weak().small());
    }
    section(ui, "Direção");
    let mut custom = h.dir.is_some();
    if crate::editor::check(ui, &mut custom, "por uma linha, aresta ou eixo (vetor)").changed() {
        h.dir = custom.then(|| seed.curves.first().map(crate::geo_ui::axis_of).unwrap_or(AxisRef::Origin(OriginAxis::Z)));
    }
    if let Some(a) = &mut h.dir {
        crate::geo_ui::axis_slot(ui, "furo_dir", "Vetor", doc, Id(u64::MAX), a, seed);
    } else {
        let txt = if h.points.is_empty() { "normal ao plano do sketch" } else { "normal à superfície em cada ponto" };
        ui.label(RichText::new(tr(txt)).weak().small());
    }
    crate::editor::check(ui, &mut h.reverse, "inverter sentido");

    // estado atual: "norma|designação|folga" (formato antigo "M6 passagem normal" também é lido)
    let st = h.standard.clone().unwrap_or_default();
    let parts: Vec<&str> = st.split('|').collect();
    let (mut norma, mut size, mut folga) = if parts.len() == 3 {
        (parts[0].to_string(), parts[1].to_string(), parts[2].to_string())
    } else {
        let m = st.split_whitespace().next().unwrap_or("M6");
        let name = hardware::THREADS.iter().find(|t| t.name.starts_with(&format!("{m}x"))).map(|t| t.name).unwrap_or("M6x1");
        let f = if st.contains("fino") { "fino" } else if st.contains("folgado") { "folgado" } else { "normal" };
        ("ISO métrica grossa".to_string(), name.to_string(), f.to_string())
    };
    let tapped = h.thread.is_some();
    let head_idx = match &h.kind {
        HoleKind::Rebaixado { .. } => 1,
        HoleKind::Escareado { .. } => 2,
        HoleKind::Contrafurado { .. } => 4,
        _ => 0,
    };
    let before = (norma.clone(), size.clone(), folga.clone(), head_idx, tapped);

    section(ui, "Boca");
    let opts = [
        ("furo_simples", "Simples", "Sem rebaixo", head_idx == 0),
        ("furo_rebaixado", "Rebaixado", "Rebaixo para cabeça cilíndrica (Allen)", head_idx == 1),
        ("furo_escareado", "Escareado", "Escareado para cabeça chata", head_idx == 2),
        ("furo_contrafurado", "Contrafurado", "Rebaixo com fundo cônico (counterdrilled)", head_idx == 4),
    ];
    let new_head = match icon_choice(ui, &opts) {
        Some(3) => 4,
        Some(i) => i,
        None => head_idx,
    };
    section(ui, "Rosca");
    let opts = [
        ("furo_simples", "Passagem", "Sem rosca: diâmetro de passagem pela folga", !tapped),
        ("furo_roscado", "Cosmética", "Broca de rosca + indicação da rosca (leve; vale no desenho e na BOM)", tapped && !h.thread_modeled),
        ("furo_roscado", "Modelada", "Broca de rosca + sulco helicoidal real (mais pesado)", tapped && h.thread_modeled),
    ];
    let mut new_tapped = tapped;
    match icon_choice(ui, &opts) {
        Some(0) => new_tapped = false,
        Some(1) => {
            new_tapped = true;
            h.thread_modeled = false;
        }
        Some(2) => {
            new_tapped = true;
            h.thread_modeled = true;
        }
        _ => {}
    }
    if new_tapped {
        crate::editor::check(ui, &mut h.left_hand, "rosca à esquerda");
    }

    section(ui, "Rosca / fixador");
    ui.horizontal(|ui| {
        ui.add_sized([60., 18.], egui::Label::new(tr("Norma")));
        egui::ComboBox::from_id_salt("norma_rosca").selected_text(norma.clone()).width(190.).show_ui(ui, |ui| {
            for s in hardware::STANDARDS {
                if ui.selectable_label(norma == s, s).clicked() && norma != s {
                    norma = s.to_string();
                    size = hardware::threads_of(s).next().map(|t| t.name.to_string()).unwrap_or_default();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        ui.add_sized([60., 18.], egui::Label::new(tr("Tamanho")));
        egui::ComboBox::from_id_salt("tam_rosca").selected_text(size.clone()).width(190.).show_ui(ui, |ui| {
            for t in hardware::threads_of(&norma) {
                ui.selectable_value(&mut size, t.name.to_string(), t.name);
            }
        });
    });
    if !new_tapped {
        ui.horizontal(|ui| {
            ui.add_sized([60., 18.], egui::Label::new(tr("Folga")));
            for f in ["fino", "normal", "folgado"] {
                ui.radio_value(&mut folga, f.to_string(), f);
            }
        });
    }

    // aplica a escolha (só quando algo mudou, para não sobrescrever ajustes finos)
    if (norma.clone(), size.clone(), folga.clone(), new_head, new_tapped) != before {
        let spec = hardware::thread(&size).copied();
        let metric = spec.and_then(|t| hardware::METRIC.iter().find(|m| (t.major - m.name[1..].parse::<f64>().unwrap_or(0.)).abs() < 1e-6));
        let fi = match folga.as_str() {
            "fino" => 0,
            "folgado" => 2,
            _ => 1,
        };
        if let Some(t) = spec {
            let clearance = metric.map(|m| m.clearance[fi]).unwrap_or(t.major * [1.05, 1.1, 1.2][fi]);
            h.diameter = format!("{}", if new_tapped { t.tap_drill } else { (clearance * 100.).round() / 100. });
            h.thread = new_tapped.then(|| t.name.to_string());
            h.kind = match new_head {
                1 => {
                    let (d, p) = metric.map(|m| (m.cbore_diameter, m.cbore_depth)).unwrap_or((t.major * 1.8, t.major));
                    HoleKind::Rebaixado { diameter: d.to_string(), depth: p.to_string() }
                }
                2 => HoleKind::Escareado { diameter: metric.map(|m| m.csink_diameter).unwrap_or(t.major * 2.).to_string(), angle: "90".into() },
                4 => {
                    let (d, p) = metric.map(|m| (m.cbore_diameter, m.cbore_depth)).unwrap_or((t.major * 1.8, t.major));
                    HoleKind::Contrafurado { diameter: d.to_string(), depth: p.to_string(), angle: "90".into() }
                }
                _ => HoleKind::Simples,
            };
        }
        h.standard = Some(format!("{norma}|{size}|{folga}"));
    }

    extent_ui(ui, "ext_furo", &mut h.depth, vars, &["Cego", "Passante", "Até o próximo", "Até o último", "Até plano", "Até a superfície"], doc, seed);


    section(ui, "Fundo e corpo");
    let mut v = h.tip.is_some();
    if crate::editor::check(ui, &mut v, "fundo em V (ponta da broca)").changed() {
        h.tip = v.then(|| "118".to_string());
    }
    if let Some(t) = &mut h.tip {
        expr(ui, "Ângulo da ponta", t, vars, "°");
    }
    expr(ui, "Conicidade", &mut h.taper, vars, "°");
    ui.label(RichText::new(tr("Conicidade ≠ 0 faz um furo cônico (meio-ângulo); 0 = cilíndrico.")).weak().small());

    egui::CollapsingHeader::new(tr("Ajuste fino das dimensões")).default_open(false).show(ui, |ui| {
        expr(ui, "Diâmetro", &mut h.diameter, vars, "mm");
        match &mut h.kind {
            HoleKind::Simples => {}
            HoleKind::Rebaixado { diameter, depth } => {
                expr(ui, "Ø rebaixo", diameter, vars, "mm");
                expr(ui, "Prof. rebaixo", depth, vars, "mm");
            }
            HoleKind::Escareado { diameter, angle } => {
                expr(ui, "Ø escareado", diameter, vars, "mm");
                expr(ui, "Ângulo", angle, vars, "°");
            }
            HoleKind::Contrafurado { diameter, depth, angle } => {
                expr(ui, "Ø contrafuro", diameter, vars, "mm");
                expr(ui, "Prof. contrafuro", depth, vars, "mm");
                expr(ui, "Ângulo do fundo", angle, vars, "°");
            }
        }
        if let Some(t) = &mut h.thread {
            ui.horizontal(|ui| {
                ui.add_sized([96., 18.], egui::Label::new(tr("Designação")));
                ui.text_edit_singleline(t);
            });
        }
    });
    target_combo(ui, doc, target);
}
