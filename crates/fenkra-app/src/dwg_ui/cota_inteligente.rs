// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cota inteligente (B107): uma ferramenta só. O que está sob o cursor (arco, aresta, vértice)
//! propõe o tipo; Tab ou a barra trocam entre os tipos que servem; a cota viva (fantasma) segue o
//! mouse até o clique de afastamento; a ferramenta continua ativa até Esc ou clique direito. Sem
//! âncora sob o cursor, o cursor mostra o motivo. A escolha do tipo e a montagem da cota vivem em
//! `fenkra_desenho::cotas`; aqui só se pega o alvo na folha e se desenha.

use super::*;
use fenkra_desenho::cotas::{self, Alvo, Motivo};

/// Estado da ferramenta entre os quadros.
#[derive(Default)]
pub struct Estado {
    /// alvos já escolhidos (1 para arco/aresta, 2 para dois pontos ou duas arestas)
    pub sel: Vec<Alvo>,
    /// vista dos alvos
    pub view: Option<Id>,
    /// tipo trocado por Tab ou pela barra (None = o proposto)
    pub tipo: Option<DimKind>,
    /// usa a chamada do furo (gerada das features) como texto da cota de diâmetro
    pub chamada: bool,
}

impl Estado {
    pub fn limpar(&mut self) {
        *self = Estado::default();
    }
}

/// Tipo em vigor: o escolhido, se ainda serve à seleção, ou o proposto.
pub(super) fn tipo_atual(ci: &Estado) -> Option<DimKind> {
    let lista = cotas::tipos(&ci.sel);
    ci.tipo.filter(|t| lista.contains(t)).or_else(|| lista.first().copied())
}

fn dist(a: V2, b: V2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Distância do ponto `p` ao segmento a–b (mm da folha).
fn dist_seg(p: V2, a: V2, b: V2) -> f64 {
    let (d, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 < 1e-12 { 0. } else { ((ap[0] * d[0] + ap[1] * d[1]) / l2).clamp(0., 1.) };
    dist(p, [a[0] + d[0] * t, a[1] + d[1] * t])
}

/// Trecho reto de aresta mais perto do cursor (a menos de 8 px): vista e extremos na vista.
fn aresta_perto(st: &mut DwgState, m: V2, so: Option<Id>) -> Option<(Id, V2, V2)> {
    let tol = 8. / st.zoom as f64;
    let mut melhor: Option<(f64, Id, V2, V2)> = None;
    let views: Vec<View> = st.d.views.iter().filter(|v| v.sheet == st.sheet && so.is_none_or(|x| x == v.id)).cloned().collect();
    for v in &views {
        let p = st.models.proj(&st.d, v.id);
        for (pl, k) in &p.lines {
            // só arestas visíveis e de contorno: retas de dois pontos
            if pl.len() != 2 || !matches!(k, 0 | 2) {
                continue;
            }
            let d = dist_seg(m, layout::to_sheet(v, &p, pl[0]), layout::to_sheet(v, &p, pl[1]));
            if d < tol && melhor.as_ref().is_none_or(|b| d < b.0) {
                melhor = Some((d, v.id, pl[0], pl[1]));
            }
        }
    }
    melhor.map(|b| (b.1, b.2, b.3))
}

/// O que há sob o cursor, ou por que não há nada. Vértice colado no cursor vence; depois arco,
/// vértice mais longe e aresta.
pub(super) fn alvo_sob(st: &mut DwgState, m: V2, snap: &SnapFn, view_at: &ViewAt) -> Result<(Id, Alvo), Motivo> {
    let so = st.ci.view;
    let sobre = view_at(st, m);
    if sobre.is_none() && so.is_none() {
        return Err(Motivo::ForaDeVista);
    }
    let px = 1. / st.zoom as f64;
    let perto = snap(st, m, so);
    if let Some((vid, s, q)) = perto {
        if dist(s, m) <= 5. * px {
            return Ok((vid, Alvo::Ponto(q)));
        }
    }
    if let Some((vid, c, on)) = ghost::arc_near(st, m).filter(|(v, ..)| so.is_none_or(|x| x == *v)) {
        return Ok((vid, Alvo::Arco { centro: c, ponto: on }));
    }
    if let Some((vid, _, q)) = perto {
        return Ok((vid, Alvo::Ponto(q)));
    }
    if let Some((vid, a, b)) = aresta_perto(st, m, so) {
        return Ok((vid, Alvo::Aresta { a, b }));
    }
    if sobre.is_some_and(|v| so.is_some_and(|x| x != v)) {
        return Err(Motivo::OutraVista);
    }
    let vid = so.or(sobre).unwrap_or(Id(0));
    if st.models.proj(&st.d, vid).axes.is_none() {
        return Err(Motivo::VistaSemProjecao);
    }
    Err(Motivo::SemAresta)
}

/// Cota do tipo `tipo` para `sel` na vista `vid`, com o afastamento que o cursor `m` pede.
pub(super) fn montar(st: &mut DwgState, vid: Id, sel: &[Alvo], tipo: DimKind, m: V2, id: Id) -> Option<Dim> {
    let v = st.d.view(vid)?.clone();
    let p = st.models.proj(&st.d, vid);
    let ax = p.axes?;
    let pts: Vec<V2> = cotas::pontos(sel, tipo)?.into_iter().map(|q| layout::to_sheet(&v, &p, q)).collect();
    let mut dm = cotas::montar(vid, &ax, sel, tipo, cotas::afastamento(tipo, &pts, m))?;
    dm.id = id;
    if st.ci.chamada && tipo == DimKind::Diameter {
        if let Some(t) = chamada_do_furo(st, vid, sel) {
            dm.text = t;
        }
    }
    Some(dm)
}

/// Chamada do furo (`4× Ø6,5 ▽12`) se o arco selecionado é um furo da peça de origem.
pub(super) fn chamada_do_furo(st: &mut DwgState, vid: Id, sel: &[Alvo]) -> Option<String> {
    let [Alvo::Arco { centro, ponto }] = sel else { return None };
    let src = st.d.view(vid)?.source.clone();
    let d = 2. * dist(*centro, *ponto);
    st.models.chamada_furo(&src, d).map(|c| c.text)
}

/// Barra da ferramenta: tipos possíveis para a seleção e a chamada do furo.
fn barra(ctx: &egui::Context, st: &mut DwgState, rect: Rect) {
    let lista = cotas::tipos(&st.ci.sel);
    let atual = tipo_atual(&st.ci);
    let chamada = st.ci.view.and_then(|v| {
        let sel = st.ci.sel.clone();
        chamada_do_furo(st, v, &sel)
    });
    egui::Area::new(egui::Id::unique("dwg_barra_cota_inteligente")).order(egui::Order::Foreground).fixed_pos(rect.left_top() + Vec2::new(12., 44.)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Cota inteligente")).strong());
                ui.separator();
                if lista.is_empty() {
                    ui.label(RichText::new(tr("Clique um arco, uma aresta ou dois vértices")).weak());
                }
                for k in &lista {
                    if ui.selectable_label(atual == Some(*k), tr(k.label())).on_hover_text(tr("Tab troca o tipo da cota")).clicked() {
                        st.ci.tipo = Some(*k);
                    }
                }
                if let Some(t) = &chamada {
                    ui.separator();
                    crate::editor::check(ui, &mut st.ci.chamada, "Chamada do furo");
                    ui.label(RichText::new(t).weak());
                }
            });
        });
    });
}

/// Um quadro da ferramenta: Tab, botão direito, alvo sob o cursor, cota viva e clique.
#[allow(clippy::too_many_arguments)]
pub(super) fn tratar(ui: &egui::Ui, painter: &egui::Painter, st: &mut DwgState, resp: &egui::Response, mouse: Option<V2>, to_scr: &dyn Fn(V2) -> Pos2, snap: &SnapFn, view_at: &ViewAt, rect: Rect) {
    let cab = if st.ci.sel.is_empty() { "Passe o mouse num arco, aresta ou vértice e clique" } else if cotas::completa(&st.ci.sel) { "Clique onde a cota fica (Tab troca o tipo; clique direito cancela)" } else { "Clique o segundo vértice" };
    painter.text(rect.center_top() + Vec2::new(0., 10.), egui::Align2::CENTER_TOP, format!("{}   (Esc {})", tr(cab), tr("cancela")), egui::FontId::proportional(14.), ui.visuals().warn_fg_color);
    barra(ui.ctx(), st, rect);
    // Tab troca o tipo
    if !ui.ctx().egui_wants_keyboard_input() && ui.ctx().input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
        let lista = cotas::tipos(&st.ci.sel);
        if let Some(a) = tipo_atual(&st.ci) {
            st.ci.tipo = Some(cotas::proximo(&lista, a));
        }
    }
    // clique direito: cancela a seleção; sem seleção, encerra a ferramenta
    if resp.secondary_clicked() {
        if st.ci.sel.is_empty() {
            st.tool = None;
        }
        st.ci.limpar();
        return;
    }
    let Some(m) = mouse else { return };
    let accent = crate::theme::accent(ui.visuals());
    let achado = alvo_sob(st, m, snap, view_at);
    // marca do alvo e motivo no cursor
    match &achado {
        Ok((vid, alvo)) => marcar(painter, st, *vid, alvo, to_scr, accent),
        Err(motivo) => {
            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
            let g = painter.layout_no_wrap(format!("⛔ {}", tr(motivo.texto())), egui::FontId::proportional(12.), ui.visuals().text_color());
            let r = Rect::from_min_size(to_scr(m) + Vec2::new(14., 14.), g.size() + Vec2::new(10., 4.));
            painter.rect_filled(r, 3., ui.visuals().window_fill);
            painter.rect_stroke(r, 3., Stroke::new(1., ui.visuals().warn_fg_color), egui::StrokeKind::Inside);
            painter.galley(r.min + Vec2::new(5., 2.), g, ui.visuals().text_color());
        }
    }
    if let Some(v) = st.ci.view {
        for a in st.ci.sel.clone() {
            marcar(painter, st, v, &a, to_scr, accent);
        }
    }
    // seleção que a cota viva usaria: a já escolhida, ou a do cursor (prévia antes do clique)
    let hover = achado.as_ref().ok().copied();
    let mut sel = st.ci.sel.clone();
    let mut vid = st.ci.view;
    if let Some((hv, ha)) = hover {
        let angulo = matches!((sel.as_slice(), ha), ([Alvo::Aresta { a, b }], Alvo::Aresta { a: c, b: d }) if Some(hv) == vid && !paralelas(*a, *b, c, d));
        if sel.is_empty() {
            sel = vec![ha];
            vid = Some(hv);
        } else if angulo {
            sel.push(ha);
        } else if matches!(sel.as_slice(), [Alvo::Ponto(_)]) && matches!(ha, Alvo::Ponto(_)) && Some(hv) == vid {
            sel.push(ha);
        }
    }
    let (Some(viva), true) = (vid, cotas::completa(&sel)) else {
        // um vértice sozinho: linha elástica até o cursor
        if let (Some(Alvo::Ponto(_)), Some(v)) = (st.ci.sel.first(), st.ci.view) {
            if let Some(p) = st.d.view(v).cloned().map(|vw| (vw, st.models.proj(&st.d, v))) {
                if let Some(Alvo::Ponto(q)) = st.ci.sel.first() {
                    painter.line_segment([to_scr(layout::to_sheet(&p.0, &p.1, *q)), to_scr(m)], Stroke::new(1., accent));
                }
            }
        }
        if resp.clicked_by(egui::PointerButton::Primary) {
            aceitar(st, hover);
        }
        return;
    };
    let tipo = if sel.len() == st.ci.sel.len() { tipo_atual(&st.ci) } else { cotas::tipos(&sel).first().copied() };
    if let Some(tipo) = tipo {
        if let Some(dm) = montar(st, viva, &sel, tipo, m, Id(1 << 40)) {
            let g = ghost::Ghost { items: layout::dim_items(&st.d, &dm, &mut st.models), guides: vec![] };
            ghost::paint(ui, painter, &g, to_scr, st.zoom);
        }
    }
    if resp.clicked_by(egui::PointerButton::Primary) {
        let completa = cotas::completa(&st.ci.sel);
        // a cota só é fixada quando a seleção já estava completa antes do clique, a menos que o
        // clique tenha acrescentado o segundo alvo (ângulo ou dois vértices)
        if sel.len() > st.ci.sel.len() {
            aceitar(st, hover);
        } else if completa {
            if let Some(tipo) = tipo_atual(&st.ci) {
                let id = st.d.alloc();
                let sel = st.ci.sel.clone();
                if let Some(dm) = montar(st, viva, &sel, tipo, m, id) {
                    st.d.dims.push(dm);
                    // a ferramenta segue ativa para a próxima cota
                    let chamada = st.ci.chamada;
                    st.ci.limpar();
                    st.ci.chamada = chamada;
                }
            }
        } else {
            aceitar(st, hover);
        }
    }
}

fn paralelas(a: V2, b: V2, c: V2, d: V2) -> bool {
    let (r, s) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = (r[0] * s[1] - r[1] * s[0]).abs();
    den <= 1e-6 * r[0].hypot(r[1]) * s[0].hypot(s[1])
}

/// Acrescenta o alvo sob o cursor à seleção (primeiro clique, segundo vértice ou segunda aresta).
fn aceitar(st: &mut DwgState, hover: Option<(Id, Alvo)>) {
    let Some((vid, alvo)) = hover else { return };
    match (st.ci.sel.as_slice(), alvo) {
        ([], _) => {
            st.ci.sel = vec![alvo];
            st.ci.view = Some(vid);
            st.ci.tipo = None;
        }
        ([Alvo::Ponto(_)], Alvo::Ponto(_)) if st.ci.view == Some(vid) => st.ci.sel.push(alvo),
        ([Alvo::Aresta { a, b }], Alvo::Aresta { a: c, b: d }) if st.ci.view == Some(vid) && !paralelas(*a, *b, c, d) => {
            st.ci.sel.push(alvo);
            st.ci.tipo = None;
        }
        _ => {}
    }
}

/// Realça o alvo na folha com a cor de destaque do tema.
fn marcar(painter: &egui::Painter, st: &mut DwgState, vid: Id, alvo: &Alvo, to_scr: &dyn Fn(V2) -> Pos2, cor: Color32) {
    let Some(v) = st.d.view(vid).cloned() else { return };
    let p = st.models.proj(&st.d, vid);
    let s = |q: V2| to_scr(layout::to_sheet(&v, &p, q));
    match alvo {
        Alvo::Ponto(q) => {
            painter.circle_stroke(s(*q), 5.5, Stroke::new(2., cor));
        }
        Alvo::Aresta { a, b } => {
            painter.line_segment([s(*a), s(*b)], Stroke::new(3., cor.gamma_multiply(0.8)));
        }
        Alvo::Arco { centro, ponto } => {
            let r = dist(*centro, *ponto);
            let pts: Vec<Pos2> = fenkra_desenho::source::circle(*centro, r).into_iter().map(s).collect();
            painter.add(egui::Shape::line(pts, Stroke::new(3., cor.gamma_multiply(0.8))));
        }
    }
}
