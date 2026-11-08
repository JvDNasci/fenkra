// SPDX-License-Identifier: LGPL-2.1-or-later
//! Fantasma de posicionamento (B224): ao armar uma ferramenta, o que ela vai criar aparece preso ao
//! cursor, na escala real (contorno e projeção da vista, tabela com as colunas, cota viva com o valor
//! medido, balão com o número do item), com linhas de alinhamento tracejadas. Nada entra no desenho
//! antes da confirmação (clique ou Enter): o fantasma é montado num desenho de rascunho, e o clique
//! de verdade usa as mesmas funções, então o que se vê é o que será criado.

use super::*;
use fenkra_desenho::source::TableData;

/// Ids do rascunho partem daqui: nunca colidem com os do desenho real (o cache de projeções é por id).
pub(super) const SCRATCH: u64 = 1 << 40;

pub struct Ghost {
    /// primitivas na folha (mm)
    pub items: Vec<layout::Item>,
    /// linhas de alinhamento tracejadas (mm na folha)
    pub guides: Vec<(V2, V2)>,
}

/// Neste clique a ferramenta escolhe a posição final do objeto (e por isso alinha)?
pub fn places(tool: u8, n: usize) -> bool {
    match tool {
        c::SET | c::VIEW | c::ISO | c::FLAT | c::SCHEMA | c::VIEW_CAM | c::NOTE | c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT => true,
        // a linha do corte e o contorno do detalhe têm posicionamento próprio (`corte_ui`)
        c::SECTION | c::DETAIL => false,
        c::BALLOON | c::ROUGH | c::WELD | c::GDT | c::DATUM => n == 1,
        _ => false,
    }
}

/// Posição que o clique vai usar: a do cursor, alinhada com vistas e balões vizinhos.
pub fn placed(st: &DwgState, tool: u8, m: V2) -> V2 {
    if places(tool, st.clicks.len()) {
        align(st, m).0
    } else {
        m
    }
}

/// Alinha `m` com bordas e centros das vistas da folha e com os balões vizinhos (dentro de 8 px).
/// Devolve o ponto ajustado e as guias (da referência ao ponto).
pub fn align(st: &DwgState, m: V2) -> (V2, Vec<(V2, V2)>) {
    let tol = (8. / st.zoom) as f64;
    let (mut xs, mut ys): (Vec<(f64, f64)>, Vec<(f64, f64)>) = (vec![], vec![]);
    for (id, lo, hi) in &st.boxes {
        if st.d.view(*id).is_some_and(|v| v.sheet == st.sheet) {
            let c = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
            xs.extend([lo[0], c[0], hi[0]].map(|x| (x, c[1])));
            ys.extend([lo[1], c[1], hi[1]].map(|y| (y, c[0])));
        }
    }
    for n in st.d.notes.iter().filter(|n| n.sheet == st.sheet && n.kind == NoteKind::Balloon) {
        xs.push((n.pos[0], n.pos[1]));
        ys.push((n.pos[1], n.pos[0]));
    }
    let best = |v: &[(f64, f64)], at: f64| v.iter().filter(|(a, _)| (a - at).abs() <= tol).min_by(|a, b| (a.0 - at).abs().total_cmp(&(b.0 - at).abs())).copied();
    let (bx, by) = (best(&xs, m[0]), best(&ys, m[1]));
    let out = [bx.map_or(m[0], |b| b.0), by.map_or(m[1], |b| b.0)];
    let mut guides = vec![];
    if let Some((x, y0)) = bx {
        guides.push(([x, y0], out));
    }
    if let Some((y, x0)) = by {
        guides.push(([x0, y], out));
    }
    (out, guides)
}

/// Desenho de rascunho: as folhas e vistas do real (para as vistas mãe existirem) e ids altos. Cada
/// finalidade (`slot`) tem a sua faixa de ids, para o cache de projeções não trocar de vista a cada
/// chamada.
pub(super) fn scratch(st: &DwgState, slot: u64) -> Drawing {
    let mut t = Drawing::new("", Format::A3);
    t.sheets = st.d.sheets.clone();
    t.views = st.d.views.clone();
    t.text_height = st.d.text_height;
    t.decimals = st.d.decimals;
    t.projection = st.d.projection;
    t.reserve_ids(SCRATCH + slot * 1000);
    t
}

/// Estilo escolhido na barra de opções (renderização, ocultas, rótulo) nas vistas novas.
pub(super) fn apply_style(st: &DwgState, t: &mut Drawing, ids: &[Id]) {
    for (k, id) in ids.iter().enumerate() {
        if let Some(v) = t.views.iter_mut().find(|v| v.id == *id) {
            v.render = st.new_render;
            v.hidden_lines = st.new_hidden;
            if k == 0 && !st.new_label.trim().is_empty() {
                v.name = st.new_label.trim().to_string();
            }
        }
    }
}

/// As vistas que a ferramenta criaria em `at`, num desenho de rascunho (e os ids delas).
pub fn new_views(st: &mut DwgState, tool: u8, at: V2) -> Option<(Drawing, Vec<Id>)> {
    let src = st.source.clone()?;
    let sc = options::scale_for(st, tool);
    let mut t = scratch(st, 0);
    let sheet = st.sheet;
    let ids = match tool {
        c::SET => {
            // B412: a mesma conta do conjunto real (conjunto::criar_sem_sobrepor): frente, lateral,
            // superior e isométrica não se encostam, na escala escolhida
            let mut g = scratch(st, 1);
            let gap = fenkra_desenho::conjunto::distancia(&mut g, &mut st.models, sheet, &src, sc, 25.).max(40.);
            t.add_standard_set(sheet, src, at, sc, gap)
        }
        c::ISO => vec![t.add_view(sheet, src, ViewKind::Standard(Dir::Iso), at, sc)],
        c::FLAT => vec![t.add_view(sheet, src, ViewKind::Flat, at, sc)],
        c::SCHEMA => vec![t.add_view(sheet, src, ViewKind::Schematic(st.new_dir), at, sc)],
        c::VIEW_CAM => {
            let (toward, x) = st.cam.unwrap_or(([0., -1., 0.], [1., 0., 0.]));
            vec![t.add_view(sheet, src, ViewKind::Camera { toward, x }, at, sc)]
        }
        _ => vec![t.add_view(sheet, src, ViewKind::Standard(st.new_dir), at, sc)],
    };
    apply_style(st, &mut t, &ids);
    Some((t, ids))
}

/// Passa para o desenho real as vistas do rascunho (com ids novos e as mães religadas).
pub fn commit_views(st: &mut DwgState, t: &Drawing, ids: &[Id]) -> Vec<Id> {
    let map: Vec<(Id, Id)> = ids.iter().map(|o| (*o, st.d.alloc())).collect();
    for (old, new) in &map {
        let Some(mut v) = t.view(*old).cloned() else { continue };
        v.id = *new;
        if let ViewKind::Projected { parent, .. } | ViewKind::Section { parent, .. } | ViewKind::Detail { parent, .. } = &mut v.kind {
            if let Some((_, n)) = map.iter().find(|(o, _)| o == parent) {
                *parent = *n;
            }
        }
        st.d.views.push(v);
    }
    map.into_iter().map(|x| x.1).collect()
}

/// Direção da vista projetada pela posição do clique em relação à mãe (1º diedro: abaixo = superior,
/// à direita = lateral esquerda).
pub fn proj_dir(parent: &View, m: V2, projecao: fenkra_desenho::novo::Projecao) -> Dir {
    let (dx, dy) = (m[0] - parent.pos[0], m[1] - parent.pos[1]);
    let base = match &parent.kind {
        ViewKind::Standard(d) | ViewKind::Projected { dir: d, .. } => *d,
        _ => Dir::Frente,
    };
    projecao.dir_projetada(base, dx, dy)
}

/// Arco ou círculo mais perto do clique (a menos de 4 mm na folha): vista, centro e um ponto dele.
pub fn arc_near(st: &mut DwgState, m: V2) -> Option<(Id, V2, V2)> {
    let tol = 4.;
    let mut best: Option<(f64, Id, V2, V2)> = None;
    let views: Vec<View> = st.d.views.iter().filter(|v| v.sheet == st.sheet).cloned().collect();
    for v in &views {
        let p = st.models.proj(&st.d, v.id);
        let q = layout::from_sheet(v, &p, m);
        for (pl, _) in &p.lines {
            let Some((c, r)) = layout::arc_center(pl) else { continue };
            let d = ((((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt() - r) * v.scale).abs();
            if d < tol && best.as_ref().is_none_or(|b| d < b.0) {
                // ponto do arco na direção do clique
                let u = [q[0] - c[0], q[1] - c[1]];
                let l = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1e-9);
                best = Some((d, v.id, c, [c[0] + u[0] / l * r, c[1] + u[1] / l * r]));
            }
        }
    }
    best.map(|b| (b.1, b.2, b.3))
}

/// Tipo e texto do símbolo que a ferramenta cria (o número do balão é o próximo livre).
pub fn symbol_note(st: &DwgState, tool: u8) -> (NoteKind, String) {
    match tool {
        c::NOTE => (NoteKind::Text, tr("Nota").to_string()),
        c::CENTER => (NoteKind::CenterMark, "5".to_string()),
        c::BALLOON => (NoteKind::Balloon, (st.d.notes.iter().filter(|n| n.kind == NoteKind::Balloon).count() + 1).to_string()),
        c::ROUGH => (NoteKind::Roughness, "Ra 1,6".to_string()),
        c::GDT => (NoteKind::Gdt, String::new()),
        c::DATUM => {
            let used: Vec<&str> = st.d.notes.iter().filter(|n| n.kind == NoteKind::Datum).map(|n| n.text.as_str()).collect();
            (NoteKind::Datum, ('A'..='Z').map(|c| c.to_string()).find(|l| !used.contains(&l.as_str())).unwrap_or_else(|| "A".into()))
        }
        _ => (NoteKind::Weld, "a4".to_string()),
    }
}

fn table_kind(tool: u8) -> TableKind {
    match tool {
        c::T_BOM => TableKind::Bom,
        c::T_HOLES => TableKind::Holes,
        c::T_BENDS => TableKind::Bends,
        _ => TableKind::CutList,
    }
}

/// Monta o fantasma da ferramenta armada com o cursor em `m` (mm na folha). Nunca altera o desenho.
pub fn compute(st: &mut DwgState, m: V2, snap: &SnapFn, view_at: &ViewAt) -> Option<Ghost> {
    let tool = st.tool?;
    if block_reason(st).is_some() {
        return None;
    }
    let n = st.clicks.len();
    let (at, guides) = if places(tool, n) { align(st, m) } else { (m, vec![]) };
    let mut g = Ghost { items: vec![], guides };
    match tool {
        c::SET | c::VIEW | c::ISO | c::FLAT | c::SCHEMA | c::VIEW_CAM => {
            let (mut t, ids) = new_views(st, tool, at)?;
            layout::align_views(&mut t);
            for id in &ids {
                g.items.extend(layout::view_items(&t, *id, &mut st.models));
            }
        }
        c::PROJ => {
            let parent = st.d.view(parent_of(st)?)?.clone();
            let mut t = scratch(st, 0);
            let id = t.add_view(st.sheet, parent.source.clone(), ViewKind::Projected { parent: parent.id, dir: proj_dir(&parent, at, st.d.projection) }, at, parent.scale);
            apply_style(st, &mut t, &[id]);
            layout::align_views(&mut t);
            g.guides.push((parent.pos, t.view(id)?.pos));
            g.items.extend(layout::view_items(&t, id, &mut st.models));
        }
        c::SECTION | c::DETAIL => return super::corte_ui::fantasma(st, m),
        c::DIM_H | c::DIM_V | c::DIM_A | c::DIM_ANG => {
            let need = if tool == c::DIM_ANG { 3 } else { 2 };
            if n + 1 != need {
                return Some(g);
            }
            let vid = st.tool_view?;
            let pt = snap(st, m, Some(vid)).map(|h| h.1).or_else(|| view_at(st, m).filter(|v| *v == vid).map(|_| m))?;
            let v = st.d.view(vid)?.clone();
            let p = st.models.proj(&st.d, vid);
            let ax = p.axes?;
            let anchors: Vec<V3> = st.clicks.iter().copied().chain([pt]).map(|c| ax.unproject(layout::from_sheet(&v, &p, c))).collect();
            let kind = match tool {
                c::DIM_H => DimKind::Horizontal,
                c::DIM_V => DimKind::Vertical,
                c::DIM_A => DimKind::Aligned,
                _ => DimKind::Angle,
            };
            let dm = Dim { id: Id(SCRATCH), view: vid, kind, anchors, offset: if kind == DimKind::Angle { 0. } else { 8. }, loose: true, ..Default::default() };
            g.items = layout::dim_items(&st.d, &dm, &mut st.models);
        }
        c::DIM_R | c::DIM_D => {
            let (vid, c0, on) = arc_near(st, m)?;
            let p = st.models.proj(&st.d, vid);
            let ax = p.axes?;
            let kind = if tool == c::DIM_R { DimKind::Radius } else { DimKind::Diameter };
            let dm = Dim { id: Id(SCRATCH), view: vid, kind, anchors: vec![ax.unproject(c0), ax.unproject(on)], offset: 6., loose: true, ..Default::default() };
            g.items = layout::dim_items(&st.d, &dm, &mut st.models);
        }
        c::CENTER => g.items = super::centros_ui::fantasma(st, m),
        c::NOTE | c::BALLOON | c::ROUGH | c::WELD | c::GDT | c::DATUM => {
            let (kind, text) = symbol_note(st, tool);
            let is_gdt = kind == NoteKind::Gdt;
            let leader = (n == 1 && !tool == c::NOTE).then(|| st.clicks[0]);
            let view = if tool == c::NOTE { view_at(st, at) } else { None };
            let note = Note { id: Id(SCRATCH), sheet: st.sheet, kind, pos: at, leader, text, size: st.d.text_height, view, gdt: is_gdt.then(DwgGdt::default), ..Default::default() };
            g.items = layout::note_items(&st.d, &note, &mut st.models);
        }
        c::T_BOM | c::T_HOLES | c::T_BENDS | c::T_CUT => {
            let kind = table_kind(tool);
            let data: TableData = match &st.ghost_table {
                Some((k, d)) if *k == tool => d.clone(),
                _ => {
                    let t = Table { id: Id(SCRATCH), sheet: st.sheet, kind: kind.clone(), source: st.source.clone()?, pos: at, level: Default::default(), cfg: Default::default() };
                    let d = st.models.table(&t, &st.d);
                    st.ghost_table = Some((tool, d.clone()));
                    d
                }
            };
            g.items = layout::table_items(at, kind.label(), &data, st.d.text_height);
        }
        _ => {}
    }
    Some(g)
}

/// Pinta o fantasma com a cor de seleção do tema, translúcida.
pub fn paint(ui: &egui::Ui, painter: &egui::Painter, g: &Ghost, to_scr: &dyn Fn(V2) -> Pos2, z: f32) {
    let col = ui.visuals().selection.stroke.color.gamma_multiply(0.85);
    for it in &g.items {
        match &it.prim {
            Prim::Line { pts, style } => {
                let pts: Vec<Pos2> = pts.iter().map(|p| to_scr(*p)).collect();
                let stroke = Stroke::new(style_px(*style, z).max(1.), col);
                let dash = style.dash();
                if dash.is_empty() {
                    painter.add(egui::Shape::line(pts, stroke));
                } else {
                    painter.extend(egui::Shape::dashed_line(&pts, stroke, (dash[0] as f32 * z).max(3.), (dash[1] as f32 * z).max(2.)));
                }
            }
            Prim::Fill { pts } => {
                painter.add(egui::Shape::convex_polygon(pts.iter().map(|p| to_scr(*p)).collect(), col.gamma_multiply(0.5), Stroke::NONE));
            }
            Prim::Text { pos, text, size, angle, align } => {
                let px = (*size as f32 * z / 0.72).max(1.);
                if px < 2.5 {
                    continue;
                }
                let gal = painter.layout_no_wrap(text.clone(), egui::FontId::proportional(px), col);
                let fx = match align {
                    Align::Left => 0.,
                    Align::Center => -gal.size().x / 2.,
                    Align::Right => -gal.size().x,
                };
                let th = -(*angle as f32).to_radians();
                let off = Vec2::new(fx, -px * 0.95);
                let rot = Vec2::new(off.x * th.cos() - off.y * th.sin(), off.x * th.sin() + off.y * th.cos());
                painter.add(egui::epaint::TextShape::new(to_scr(*pos) + rot, gal, col).with_angle(th));
            }
        }
    }
    let weak = ui.visuals().weak_text_color();
    for (a, b) in &g.guides {
        painter.extend(egui::Shape::dashed_line(&[to_scr(*a), to_scr(*b)], Stroke::new(1., weak), 6., 4.));
    }
}
