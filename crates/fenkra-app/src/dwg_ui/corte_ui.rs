// SPDX-License-Identifier: LGPL-2.1-or-later
//! Cortes completos e detalhe em perfil livre na folha (B106): a linha de corte com vários
//! segmentos concluída por Enter (ou duplo clique), o fantasma da vista em corte durante o traçado
//! preso ao alinhamento com a mãe (Ctrl solta), as opções do corte (profundidade, fatia, peças sem
//! corte, borda) e as alças dos vértices da linha. A matemática vive em `fenkra_desenho::corte`.

use super::*;
use fenkra_desenho::corte::{self, Borda, CorteOpcoes};

/// A ferramenta armada traça uma linha de vários pontos (corte, ou detalhe em perfil livre)?
pub(super) fn traca_linha(st: &DwgState, tool: u8) -> bool {
    tool == c::SECTION || (tool == c::DETAIL && st.detalhe_perfil)
}

/// Mínimo de pontos da linha: 2 no corte, 3 no perfil do detalhe.
fn minimo(tool: u8) -> usize {
    if tool == c::SECTION {
        2
    } else {
        3
    }
}

/// A linha já está concluída e o próximo clique posiciona a vista? O detalhe em círculo conclui sozinho
/// com o centro e o ponto do raio.
pub(super) fn pronta(st: &DwgState, tool: u8) -> bool {
    if tool == c::DETAIL && !st.detalhe_perfil {
        return st.clicks.len() >= 2;
    }
    st.corte_pronto
}

/// Frase do cabeçalho da folha para as ferramentas desta família (None = a de `hint`).
pub(super) fn dica(st: &DwgState) -> Option<&'static str> {
    let tool = st.tool?;
    if !traca_linha(st, tool) {
        return None;
    }
    let n = st.clicks.len();
    Some(match (tool == c::SECTION, st.corte_pronto, n) {
        (true, false, 0) => "Clique o primeiro ponto da linha de corte",
        (true, false, 1) => "Clique o segundo ponto da linha de corte",
        (true, false, _) => "Clique mais pontos; Enter ou duplo clique conclui a linha",
        (true, true, _) => "Clique onde a vista em corte vai ficar (Ctrl solta o alinhamento)",
        (false, false, 0) => "Clique o primeiro ponto do contorno do detalhe",
        (false, false, _) => "Clique os pontos do contorno; Enter ou duplo clique conclui",
        (false, true, _) => "Clique onde a vista de detalhe vai ficar",
    })
}

/// Enter ou duplo clique: conclui a linha se já tem pontos suficientes. `true` se concluiu.
pub(super) fn terminar(st: &mut DwgState) -> bool {
    let Some(tool) = st.tool else { return false };
    if !traca_linha(st, tool) || st.corte_pronto || st.clicks.len() < minimo(tool) {
        return false;
    }
    st.corte_pronto = true;
    true
}

fn perto(a: V2, b: V2) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 0.8
}

/// Linha sem pontos repetidos em sequência (o duplo clique repete o último).
fn sem_repeticao(pts: &[V2]) -> Vec<V2> {
    let mut out: Vec<V2> = vec![];
    for p in pts {
        if out.last().is_none_or(|q| !perto(*q, *p)) {
            out.push(*p);
        }
    }
    out
}

/// Vista (tipo, opções e escala) que a linha `pts` (mm na folha) cria sobre `parent`.
fn montar(st: &mut DwgState, tool: u8, parent: &View, pts: &[V2]) -> Option<(ViewKind, View2, f64)> {
    let p = st.models.proj(&st.d, parent.id);
    let pc: Vec<V2> = pts.iter().map(|q| layout::from_sheet(parent, &p, *q)).collect();
    let label = st.next_label();
    let sc = options::scale_for(st, tool);
    if tool == c::SECTION {
        if pc.len() < 2 {
            return None;
        }
        let mut op = st.corte_novo.clone();
        op.pontos = pc[2..].to_vec();
        return Some((ViewKind::Section { parent: parent.id, a: pc[0], b: pc[1], label }, View2::Corte(op), sc));
    }
    if st.detalhe_perfil {
        if pc.len() < 3 {
            return None;
        }
        let n = pc.len() as f64;
        let centro = pc.iter().fold([0., 0.], |a, q| [a[0] + q[0] / n, a[1] + q[1] / n]);
        let raio = pc.iter().map(|q| (q[0] - centro[0]).hypot(q[1] - centro[1])).fold(1., f64::max);
        return Some((ViewKind::Detail { parent: parent.id, center: centro, radius: raio, label }, View2::Perfil(pc), sc));
    }
    let (a, b) = (pc.first()?, pc.get(1)?);
    let r = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt().max(1.);
    Some((ViewKind::Detail { parent: parent.id, center: *a, radius: r, label }, View2::Nada, sc))
}

/// O que acompanha o `ViewKind` na criação.
enum View2 {
    Corte(CorteOpcoes),
    Perfil(Vec<V2>),
    Nada,
}

/// Posição da vista nova com o cursor em `m`: o corte desliza só na perpendicular à linha (Ctrl
/// solta); o detalhe alinha com as vistas vizinhas.
fn posicao(st: &DwgState, tool: u8, pts: &[V2], parent: &View, m: V2) -> V2 {
    if tool == c::SECTION && pts.len() >= 2 {
        if let Some(dir) = corte::direcao_de_alinhamento(pts[0], pts[1]) {
            return corte::alinhar_ao_corte(parent.pos, dir, m, st.ctrl_livre);
        }
    }
    ghost::align(st, m).0
}

/// Clique da ferramenta: soma um ponto à linha, conclui por duplo clique ou, com a linha pronta,
/// cria a vista. Devolve a vista criada.
pub(super) fn clique(st: &mut DwgState, m: V2) -> Option<Id> {
    let tool = st.tool?;
    let parent = st.cur().and_then(|s| st.d.view(s).cloned())?;
    if !pronta(st, tool) {
        if st.clicks.last().is_some_and(|q| perto(*q, m)) {
            // segundo clique do duplo clique
            terminar(st);
            return None;
        }
        st.clicks.push(m);
        return None;
    }
    let pts = sem_repeticao(&st.clicks);
    let (kind, extra, sc) = montar(st, tool, &parent, &pts)?;
    let at = posicao(st, tool, &pts, &parent, m);
    let id = st.d.add_view(st.sheet, parent.source.clone(), kind, at, sc);
    if let Some(v) = st.d.views.iter_mut().find(|v| v.id == id) {
        match extra {
            View2::Corte(op) => v.corte = op,
            View2::Perfil(c) => v.detalhe.contorno = c,
            View2::Nada => {}
        }
    }
    style_new(st, id);
    st.tool = None;
    st.clicks.clear();
    st.tool_view = None;
    st.loose = false;
    st.corte_pronto = false;
    st.sel = Some(id);
    Some(id)
}

/// Fantasma durante o traçado e o posicionamento: o contorno da linha e, havendo ao menos dois
/// pontos, a vista resultante (hachura incluída).
pub(super) fn fantasma(st: &mut DwgState, m: V2) -> Option<ghost::Ghost> {
    let tool = st.tool?;
    let parent = st.d.view(parent_of(st)?)?.clone();
    let pronto = pronta(st, tool);
    let mut pts = st.clicks.clone();
    if !pronto {
        // o vértice provisório anda em passos de 1 mm: a projeção do corte só refaz quando muda
        pts.push([m[0].round(), m[1].round()]);
    }
    let pts = sem_repeticao(&pts);
    let mut g = ghost::Ghost { items: vec![], guides: vec![] };
    if traca_linha(st, tool) {
        let cor = layout::Item { owner: None, prim: layout::Prim::Line { pts: pts.clone(), style: Style::Center }, color: None, width: None, bold: false, alert: false };
        g.items.push(cor);
        if tool == c::DETAIL && pts.len() >= 3 {
            let mut f = pts.clone();
            f.push(pts[0]);
            g.items.push(layout::Item { owner: None, prim: layout::Prim::Line { pts: f, style: Style::Thin }, color: None, width: None, bold: false, alert: false });
        }
    } else if tool == c::DETAIL && st.clicks.len() == 1 {
        let r = ((m[0] - st.clicks[0][0]).powi(2) + (m[1] - st.clicks[0][1]).powi(2)).sqrt();
        g.items.push(layout::Item { owner: None, prim: layout::Prim::Line { pts: fenkra_desenho::source::circle(st.clicks[0], r), style: Style::Thin }, color: None, width: None, bold: false, alert: false });
        return Some(g);
    }
    let minimo = if tool == c::SECTION { 2 } else if st.detalhe_perfil { 3 } else { 2 };
    if pts.len() < minimo {
        return Some(g);
    }
    let base = if tool == c::DETAIL && !st.detalhe_perfil { st.clicks.clone() } else { pts.clone() };
    let (kind, extra, sc) = montar(st, tool, &parent, &base)?;
    let mut t = ghost::scratch(st, 0);
    let id = t.add_view(st.sheet, parent.source.clone(), kind, [0., 0.], sc);
    if let Some(v) = t.views.iter_mut().find(|v| v.id == id) {
        match extra {
            View2::Corte(op) => v.corte = op,
            View2::Perfil(c) => v.detalhe.contorno = c,
            View2::Nada => {}
        }
    }
    ghost::apply_style(st, &mut t, &[id]);
    let pr = st.models.proj(&t, id);
    let at = if pronto {
        posicao(st, tool, &pts, &parent, m)
    } else if tool == c::SECTION {
        // durante o traçado a vista fica ao lado da mãe, no lado das setas
        let dir = corte::direcao_de_alinhamento(pts[0], pts[1])?;
        let (lo, hi) = layout::view_rect(t.view(id)?, &pr);
        let tam_s = [hi[0] - lo[0], hi[1] - lo[1]];
        let tam_p = st.boxes.iter().find(|b| b.0 == parent.id).map(|b| [b.2[0] - b.1[0], b.2[1] - b.1[1]]).unwrap_or([60., 60.]);
        let ext = |t: [f64; 2]| dir[0].abs() * t[0] / 2. + dir[1].abs() * t[1] / 2.;
        let d = ext(tam_p) + ext(tam_s) + 14.;
        [parent.pos[0] + dir[0] * d, parent.pos[1] + dir[1] * d]
    } else {
        [parent.pos[0] + 90., parent.pos[1]]
    };
    if let Some(v) = t.views.iter_mut().find(|v| v.id == id) {
        v.pos = at;
    }
    g.items.extend(layout::view_items(&t, id, &mut st.models));
    if tool == c::SECTION {
        // linha de alinhamento com a mãe, tracejada
        g.guides.push((parent.pos, at));
    }
    Some(g)
}

/// Pontos já clicados da linha em andamento, ligados por segmentos até o cursor.
pub(super) fn guia(painter: &egui::Painter, st: &DwgState, to_scr: &dyn Fn(V2) -> Pos2, m: V2, cor: Color32) {
    for w in st.clicks.windows(2) {
        painter.line_segment([to_scr(w[0]), to_scr(w[1])], Stroke::new(1.4, cor));
    }
    let _ = m;
}

/// Controles de criação do corte na barra de opções (profundidade, fatia e borda).
pub(super) fn barra(ui: &mut egui::Ui, st: &mut DwgState) {
    ui.separator();
    corte_opcoes_ui(ui, &mut st.corte_novo, "barra");
}

/// Profundidade, fatia e borda: o mesmo bloco na barra de criação e no painel da vista.
fn corte_opcoes_ui(ui: &mut egui::Ui, op: &mut CorteOpcoes, salt: &str) {
    let mut por_distancia = op.profundidade.is_some();
    ui.label(tr("Profundidade"));
    egui::ComboBox::from_id_salt(("corte_prof", salt)).selected_text(if por_distancia { tr("Distância") } else { tr("Total") }).show_ui(ui, |ui| {
        ui.selectable_value(&mut por_distancia, false, tr("Total"));
        ui.selectable_value(&mut por_distancia, true, tr("Distância"));
    });
    match (por_distancia, op.profundidade) {
        (true, None) => op.profundidade = Some(10.),
        (false, Some(_)) => op.profundidade = None,
        _ => {}
    }
    if let Some(d) = op.profundidade.as_mut() {
        ui.add(crate::ui_ext::dv(d).range(0.1..=1e4).speed(0.2).suffix(" mm")).on_hover_text(tr("quanto do material atrás do plano de corte aparece"));
    }
    crate::editor::check(ui, &mut op.fatia, "Fatia");
    egui::ComboBox::from_id_salt(("corte_borda", salt)).selected_text(format!("{}: {}", tr("Borda"), tr(op.borda.label()))).show_ui(ui, |ui| {
        for b in Borda::ALL {
            ui.selectable_value(&mut op.borda, b, tr(b.label()));
        }
    });
}

/// Opções do corte já criado, no painel da vista. `nomes` são as peças da montagem de origem.
pub(super) fn painel(ui: &mut egui::Ui, corte: &mut CorteOpcoes, id: Id, nomes: &[String]) {
    ui.label(RichText::new(tr("Corte")).strong());
    corte_opcoes_ui(ui, corte, &format!("painel{}", id.0));
    ui.label(RichText::new(tr("Fatia mostra só a face cortada. Borda serrilhada desenha a face com traço de ruptura.")).weak().small());
    if corte.pontos.is_empty() {
        ui.label(RichText::new(tr("Linha reta: arraste as pontas na folha; clique no ponto do meio para criar um vértice (corte em degraus ou meio corte).")).weak().small());
    } else {
        ui.label(RichText::new(format!("{} {}", corte.pontos.len() + 2, tr("vértices na linha de corte; botão direito num vértice remove."))).weak().small());
    }
    if nomes.len() > 1 {
        egui::CollapsingHeader::new(format!("{} ({})", tr("Peças que não são cortadas"), corte.sem_corte.len())).id_salt(("sem_corte", id.0)).show(ui, |ui| {
            for (i, n) in nomes.iter().enumerate() {
                let mut inteira = corte.sem_corte.contains(&i);
                if ui.checkbox(&mut inteira, n).on_hover_text(tr("a peça aparece inteira, sem corte nem hachura")).changed() {
                    if inteira {
                        corte.sem_corte.push(i);
                    } else {
                        corte.sem_corte.retain(|x| *x != i);
                    }
                }
            }
        });
    }
}

/// Alças da linha de corte na vista mãe: vértices arrastáveis (botão direito remove), ponto do meio
/// de cada segmento (clique cria um vértice, arraste desloca o segmento inteiro).
pub(super) fn vertices(ui: &egui::Ui, painter: &egui::Painter, st: &mut DwgState, id: Id, to_scr: &dyn Fn(V2) -> Pos2, to_mm: &dyn Fn(Pos2) -> V2, z: f32) {
    let Some(v) = st.d.view(id).cloned() else { return };
    let ViewKind::Section { parent, .. } = &v.kind else { return };
    let Some(pv) = st.d.view(*parent).cloned() else { return };
    let pp = st.models.proj(&st.d, pv.id);
    let caminho = corte::caminho_da_vista(&st.d, id);
    if caminho.len() < 2 {
        return;
    }
    let cor = crate::theme::accent(ui.visuals());
    let fundo = ui.visuals().window_fill;
    let sid = ui.scope_id();
    let folha: Vec<V2> = caminho.iter().map(|q| layout::to_sheet(&pv, &pp, *q)).collect();
    let n = folha.len();
    // meio de cada segmento
    for i in 0..n - 1 {
        let meio = [(folha[i][0] + folha[i + 1][0]) / 2., (folha[i][1] + folha[i + 1][1]) / 2.];
        let c = to_scr(meio);
        painter.circle_filled(c, 4.5, fundo);
        painter.circle_stroke(c, 4.5, Stroke::new(1.5, cor));
        let r = ui.interact(Rect::from_center_size(c, Vec2::splat(18.)), sid.with(("corte_meio", id.0, i)), egui::Sense::click_and_drag()).on_hover_text(tr("Clique para criar um vértice; arraste para deslocar o segmento"));
        if r.clicked() {
            let mut novo = caminho.clone();
            novo.insert(i + 1, layout::from_sheet(&pv, &pp, meio));
            corte::definir_caminho(&mut st.d, id, &novo);
        }
        if r.drag_started() {
            alcas::begin_drag(st);
        }
        if r.dragged() && st.dragging {
            let dv = r.drag_delta();
            let d = [(dv.x / z) as f64 / pv.scale, -(dv.y / z) as f64 / pv.scale];
            let mut novo = corte::caminho_da_vista(&st.d, id);
            for k in [i, i + 1] {
                novo[k] = [novo[k][0] + d[0], novo[k][1] + d[1]];
            }
            corte::definir_caminho(&mut st.d, id, &novo);
        }
        if r.drag_stopped() {
            alcas::end_drag(st);
        }
    }
    // vértices
    for i in 0..n {
        let c = to_scr(folha[i]);
        painter.circle_filled(c, 6., cor);
        painter.circle_stroke(c, 6., Stroke::new(1.5, Color32::BLACK));
        let r = ui.interact(Rect::from_center_size(c, Vec2::splat(20.)), sid.with(("corte_vertice", id.0, i)), egui::Sense::click_and_drag()).on_hover_text(tr("Arraste para mover o ponto da linha de corte; botão direito remove"));
        if r.drag_started() {
            alcas::begin_drag(st);
        }
        if r.dragged() && st.dragging {
            if let Some(m) = r.interact_pointer_pos().map(to_mm) {
                let mut novo = corte::caminho_da_vista(&st.d, id);
                novo[i] = layout::from_sheet(&pv, &pp, m);
                corte::definir_caminho(&mut st.d, id, &novo);
            }
        }
        if r.drag_stopped() {
            alcas::end_drag(st);
        }
        if r.secondary_clicked() && n > 2 {
            let mut novo = corte::caminho_da_vista(&st.d, id);
            novo.remove(i);
            corte::definir_caminho(&mut st.d, id, &novo);
        }
    }
}

/// Vértices do contorno livre do detalhe: arrastar move o ponto; botão direito remove (mínimo de 3).
pub(super) fn vertices_detalhe(ui: &egui::Ui, painter: &egui::Painter, st: &mut DwgState, id: Id, to_scr: &dyn Fn(V2) -> Pos2, to_mm: &dyn Fn(Pos2) -> V2) {
    let Some(v) = st.d.view(id).cloned() else { return };
    let ViewKind::Detail { parent, .. } = &v.kind else { return };
    let Some(pv) = st.d.view(*parent).cloned() else { return };
    let pp = st.models.proj(&st.d, pv.id);
    let cor = crate::theme::accent(ui.visuals());
    let sid = ui.scope_id();
    for (i, q) in v.detalhe.contorno.iter().enumerate() {
        let c = to_scr(layout::to_sheet(&pv, &pp, *q));
        painter.circle_filled(c, 6., cor);
        painter.circle_stroke(c, 6., Stroke::new(1.5, Color32::BLACK));
        let r = ui.interact(Rect::from_center_size(c, Vec2::splat(20.)), sid.with(("detalhe_vertice", id.0, i)), egui::Sense::click_and_drag()).on_hover_text(tr("Arraste para mover o ponto do contorno; botão direito remove"));
        if r.drag_started() {
            alcas::begin_drag(st);
        }
        if r.dragged() && st.dragging {
            if let Some(m) = r.interact_pointer_pos().map(to_mm) {
                let mut novo = st.d.view(id).map(|x| x.detalhe.contorno.clone()).unwrap_or_default();
                if i < novo.len() {
                    novo[i] = layout::from_sheet(&pv, &pp, m);
                    corte::definir_contorno_detalhe(&mut st.d, id, novo);
                }
            }
        }
        if r.drag_stopped() {
            alcas::end_drag(st);
        }
        if r.secondary_clicked() && v.detalhe.contorno.len() > 3 {
            let mut novo = v.detalhe.contorno.clone();
            novo.remove(i);
            corte::definir_contorno_detalhe(&mut st.d, id, novo);
        }
    }
}
