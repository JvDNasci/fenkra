// SPDX-License-Identifier: LGPL-2.1-or-later
//! Detalhamento 3D (PMI): criação pelas seleções da viewport, desenho sobre o modelo (linhas de
//! cota com setas, chamadas, símbolos) e painel de edição. Peça e montagem têm a própria lista.

use crate::app::{App, Sel};
use crate::i18n::tr;
use crate::picking::Pick;
use crate::pmi_draw::{Cv, Pen, HL};
use crate::render::Camera;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use glam::Vec3;
use fenkra_core::pmi::*;
use fenkra_core::Id;
use fenkra_solidos::RecomputeResult;


/// Tipos na ordem dos comandos da barra.
pub fn kind_of(i: u8) -> PmiKind {
    match i {
        0 => PmiKind::Linear,
        1 => PmiKind::Radius,
        2 => PmiKind::Diameter,
        3 => PmiKind::Angle,
        4 => PmiKind::Note,
        5 => PmiKind::Roughness { ra: "1,6".into(), process: String::new(), extra: Default::default() },
        19 => PmiKind::DatumTarget { alvo: Default::default() },
        6 => PmiKind::Weld { kind: WeldKind::Filete, size: "a4".into(), all_around: false, extra: Default::default() },
        7 => PmiKind::Datum { letter: "A".into() },
        18 => PmiKind::Balloon { item: "1".into(), manual: true },
        9 => PmiKind::TitleBlock { template: TITLE_TEMPLATE.into() },
        11 => PmiKind::Ordinate { axis: 0 },
        12 => PmiKind::ArcLength,
        13 | 14 => PmiKind::Linear,
        15 => PmiKind::FeatureNote { feature: Id(0), comp: Id(0) },
        16 => PmiKind::Note,
        17 => PmiKind::Linear,
        _ => PmiKind::Gdt { ch: GdtChar::Planeza, tol: "0,05".into(), datums: String::new(), extra: Default::default() },
    }
}

/// Variante de medida escolhida com Tab no comando aberto (B185): vale até criar ou cancelar.
static VARIANTE: std::sync::Mutex<Option<(u8, PmiKind)>> = std::sync::Mutex::new(None);

fn set_variant(v: Option<(u8, PmiKind)>) {
    crate::pmi_smart::tab_reset();
    *VARIANTE.lock().unwrap_or_else(|e| e.into_inner()) = v;
}

/// Tipo do comando `i` com a variante escolhida na sessão.
fn kind_now(i: u8) -> PmiKind {
    match VARIANTE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        Some((j, k)) if *j == i => k.clone(),
        _ => kind_of(i),
    }
}

/// Anotação que está ganhando uma ponta nova (comando 16): o próximo elemento escolhido vira a ponta.
static PONTA_PARA: std::sync::Mutex<Option<Id>> = std::sync::Mutex::new(None);

/// Cadeia (13) e base (14) pedem vários pontos e terminam com Enter.
fn is_multi(i: u8) -> bool {
    matches!(i, 13 | 14)
}

/// Lista de PMI do nível atual (montagem aberta na bancada Montagem, senão a peça).
pub fn list(app: &App) -> &Vec<Pmi> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => &st.asm.pmi,
        None => &app.open.doc.pmi,
    }
}

pub fn list_mut(app: &mut App) -> &mut Vec<Pmi> {
    if app.in_asm() {
        &mut app.asm.as_mut().unwrap().asm.pmi
    } else {
        &mut app.open.doc.pmi
    }
}

/// Malha onde as anotações se apoiam: a vista da montagem quando ela está aberta, senão a peça.
/// Na montagem usar a malha da peça aberta comparava as âncoras com a geometria errada (realce e
/// reencaixe falhavam).
fn model(app: &App) -> Option<&RecomputeResult> {
    if app.in_asm() {
        app.asm.as_ref().and_then(|s| s.view.as_ref())
    } else {
        app.result.as_ref()
    }
}

/// Âncoras a partir das seleções: vértice → posição; aresta → ponto médio (raio/diâmetro: centro
/// e um ponto do arco); face → ponto clicado.
fn anchors(app: &App, kind: &PmiKind, picks: &[Pick]) -> Option<(Vec<[f64; 3]>, Vec<Option<PmiTarget>>)> {
    let r = model(app);
    let pt = |p: &Pick| -> Option<[f64; 3]> {
        Some(match *p {
            Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
            Pick::Edge { mid, .. } => mid.map(|v| v as f64),
            Pick::Face { point, .. } => point.map(|v| v as f64),
            _ => return None,
        })
    };
    match kind {
        PmiKind::Radius | PmiKind::Diameter => {
            // aresta circular, ou face cilíndrica (furo ou eixo): a âncora grava a chave do alvo
            let t = pick_target(r?, picks.first()?)?;
            let on = matches!(t.class, TargetClass::ArestaCircular | TargetClass::FaceCilindrica);
            if !on {
                return None;
            }
            let hit = pt(picks.first()?)?;
            let surf = match picks.first()? {
                Pick::Edge { mid, .. } => mid.map(|v| v as f64),
                _ => t.carry(&t, hit, false),
            };
            Some((vec![t.carry(&t, hit, true), surf], vec![Some(t), None]))
        }
        PmiKind::FeatureNote { .. } => {
            // uma face gerada por furo, rosca, chanfro, filete ou dobra: a âncora é o ponto clicado
            let Some(p @ Pick::Face { .. }) = picks.first() else { return None };
            let (comp, feature) = crate::pmi_feat_asm::dono_do_pick(app, p)?;
            crate::pmi_feat_asm::chamada_de(app, comp, feature, true)?;
            Some((vec![pt(p)?], vec![r.and_then(|r| pick_target(r, p))]))
        }
        PmiKind::Angle if picks.len() == 2 && picks.iter().all(|p| matches!(p, Pick::Edge { .. })) => {
            // duas arestas retas: o vértice é onde as retas se encontram (B268)
            let r = r?;
            let segs: Vec<_> = picks.iter().filter_map(|p| crate::pmi_smart::item_of(r, p)).collect();
            let (s1, s2) = (segs.first()?.seg?, segs.get(1)?.seg?);
            let a = fenkra_core::pmi_infer::angle_from_segments(s1, s2)?;
            Some((a.to_vec(), vec![None, None, None]))
        }
        PmiKind::Angle if picks.len() == 2 && picks.iter().all(|p| matches!(p, Pick::Face { .. })) => {
            // duas faces planas concorrentes: o diedro, com o vértice na reta de interseção (B268)
            let r = r?;
            let (a, b) = (pick_target(r, &picks[0])?, pick_target(r, &picks[1])?);
            if a.class != TargetClass::FacePlana || b.class != TargetClass::FacePlana {
                return None;
            }
            let v = fenkra_core::pmi_infer::angle_from_planes(pt(&picks[0])?, a.dir, pt(&picks[1])?, b.dir)?;
            // os dois alvos são as faces: a cota refaz o vértice e as pernas a cada quadro
            Some((v.to_vec(), vec![Some(a), Some(b)]))
        }
        PmiKind::Gap { .. } => {
            // dois furos, cilindros ou arestas circulares: a cota vai de eixo a eixo
            let r = r?;
            let v: Vec<([f64; 3], PmiTarget)> = picks
                .iter()
                .filter_map(|p| {
                    let t = pick_target(r, p)?;
                    matches!(t.class, TargetClass::ArestaCircular | TargetClass::FaceCilindrica).then_some((t.carry(&t, pt(p)?, true), t))
                })
                .collect();
            if v.len() < 2 {
                return None;
            }
            Some((vec![v[0].0, v[1].0], vec![Some(v[0].1), Some(v[1].1)]))
        }
        k => {
            let v: Vec<([f64; 3], Option<PmiTarget>)> = picks.iter().filter_map(|p| Some((pt(p)?, r.and_then(|r| pick_target(r, p))))).collect();
            if v.len() < k.anchors() {
                return None;
            }
            let (mut a, t): (Vec<_>, Vec<_>) = v[..k.anchors()].iter().cloned().unzip();
            // arco: se o primeiro elemento é uma aresta circular, o centro dela é o centro do arco
            if let (PmiKind::ArcLength, Some(Some(t0))) = (k, t.first()) {
                if t0.class == TargetClass::ArestaCircular {
                    a[0] = t0.point;
                }
            }
            fix_pair(kind, &mut a, &t);
            fenkra_core::pmi_cotas::project_to_axis(kind, &mut a);
            Some((a, t))
        }
    }
}

/// Cota entre duas faces planas paralelas: a segunda âncora vai para o pé da perpendicular da
/// primeira, para a linha de cota medir exatamente a distância entre os planos.
fn fix_pair(kind: &PmiKind, a: &mut [[f64; 3]], t: &[Option<PmiTarget>]) {
    if *kind != PmiKind::Linear || a.len() < 2 {
        return;
    }
    if let (Some(Some(t0)), Some(Some(t1))) = (t.first(), t.get(1)) {
        let n = t1.dir;
        let par = n.iter().zip(t0.dir).map(|(x, y)| x * y).sum::<f64>().abs() > 0.99995;
        if t0.class == TargetClass::FacePlana && t1.class == TargetClass::FacePlana && par {
            a[1] = t1.carry(t1, a[0], false);
        }
    }
}

/// Alvo (chave geométrica) do que o cursor ou a seleção apontam.
pub(crate) fn pick_target(r: &RecomputeResult, p: &Pick) -> Option<PmiTarget> {
    match *p {
        Pick::Face { body, face, .. } => face_target(crate::picking::mesh_of(r, body)?, face),
        Pick::Edge { body, edge, mid } => {
            let pts = crate::picking::edge_polyline(r, body, edge);
            let circ = fenkra_solidos::geo3d::Curve3::poly(pts, false).ok().and_then(|c| fenkra_solidos::geo3d::fit_circle(&c).ok());
            Some(match circ {
                Some((ctr, n, rad)) => PmiTarget { class: TargetClass::ArestaCircular, point: ctr, dir: n, radius: rad, concave: false },
                None => PmiTarget { class: TargetClass::Aresta, point: mid.map(|v| v as f64), dir: [0.; 3], radius: 0., concave: false },
            })
        }
        Pick::Vertex { pos, .. } => Some(PmiTarget { class: TargetClass::Vertice, point: pos.map(|v| v as f64), dir: [0.; 3], radius: 0., concave: false }),
        _ => None,
    }
}

/// Chave de uma face da malha: plano (normal externa), cilindro (eixo, raio ajustado pelos
/// vértices e sentido: furo quando a normal aponta para o eixo) ou outra superfície.
fn face_target(m: &fenkra_kernel::Mesh, face: usize) -> Option<PmiTarget> {
    let fi = m.face_info.get(face)?;
    let mk = |class, point, dir, radius, concave| Some(PmiTarget { class, point, dir, radius, concave });
    match fi.kind {
        0 => mk(TargetClass::FacePlana, fi.point, fi.dir, 0., false),
        1 => {
            let a = *m.face_ranges.get(face)? as usize;
            let b = m.face_ranges.get(face + 1).map(|&x| x as usize).unwrap_or(m.indices.len());
            let idx = &m.indices[a.min(m.indices.len())..b.min(m.indices.len())];
            let p = |i: u32| {
                let k = i as usize * 3;
                [m.positions[k] as f64, m.positions[k + 1] as f64, m.positions[k + 2] as f64]
            };
            let pts: Vec<[f64; 3]> = idx.iter().step_by(2).map(|&i| p(i)).collect();
            let (ctr, rad) = fit_axis_circle(&pts, fi.dir)?;
            let i0 = *idx.first()? as usize * 3;
            let n = [m.normals[i0] as f64, m.normals[i0 + 1] as f64, m.normals[i0 + 2] as f64];
            let to_axis = [ctr[0] - m.positions[i0] as f64, ctr[1] - m.positions[i0 + 1] as f64, ctr[2] - m.positions[i0 + 2] as f64];
            let concave = n.iter().zip(to_axis).map(|(x, y)| x * y).sum::<f64>() > 0.;
            mk(TargetClass::FaceCilindrica, ctr, fi.dir, rad, concave)
        }
        _ => mk(TargetClass::FaceOutra, fi.point, fi.dir, 0., false),
    }
}

/// Todos os alvos da geometria atual (faces, arestas circulares, meios de aresta e vértices)
/// para reencontrar os alvos das anotações. Guardado por resultado de recálculo.
fn candidates(ctx: &egui::Context, r: &RecomputeResult) -> std::sync::Arc<Vec<PmiTarget>> {
    let key = (r as *const RecomputeResult as usize, r.bodies.iter().map(|b| b.mesh.positions.len() + b.mesh.edges.len()).sum::<usize>(), r.elapsed_ms.to_bits());
    let id = egui::Id::new("pmi_candidatos");
    if let Some((k, v)) = ctx.data(|d| d.get_temp::<((usize, usize, u64), std::sync::Arc<Vec<PmiTarget>>)>(id)) {
        if k == key {
            return v;
        }
    }
    let mut out = vec![];
    for (bi, b) in r.bodies.iter().enumerate() {
        for f in 0..b.mesh.face_info.len() {
            out.extend(face_target(&b.mesh, f));
        }
        for e in 0..b.mesh.edge_mids.len() / 3 {
            let pts = crate::picking::edge_polyline(r, bi, e);
            if let Some(c) = fenkra_solidos::geo3d::Curve3::poly(pts, false).ok().and_then(|c| fenkra_solidos::geo3d::fit_circle(&c).ok()) {
                out.push(PmiTarget { class: TargetClass::ArestaCircular, point: c.0, dir: c.1, radius: c.2, concave: false });
            } else {
                let m = &b.mesh.edge_mids[e * 3..e * 3 + 3];
                out.push(PmiTarget { class: TargetClass::Aresta, point: [m[0] as f64, m[1] as f64, m[2] as f64], dir: [0.; 3], radius: 0., concave: false });
            }
        }
        for v in b.mesh.vertices.chunks_exact(3) {
            out.push(PmiTarget { class: TargetClass::Vertice, point: [v[0] as f64, v[1] as f64, v[2] as f64], dir: [0.; 3], radius: 0., concave: false });
        }
    }
    let v = std::sync::Arc::new(out);
    ctx.data_mut(|d| d.insert_temp(id, (key, v.clone())));
    v
}

/// Nome do alvo na interface ('furo Ø6,5'), traduzido.
fn target_label(t: &PmiTarget) -> String {
    let d = || num(t.radius * 2.);
    match t.class {
        TargetClass::FacePlana => tr("face plana").to_string(),
        TargetClass::FaceCilindrica => format!("{} Ø{}", tr(if t.concave { "furo" } else { "cilindro" }), d()),
        TargetClass::FaceOutra => tr("face curva").to_string(),
        TargetClass::ArestaCircular => format!("{} Ø{}", tr("aresta circular"), d()),
        TargetClass::Aresta => tr("aresta").to_string(),
        TargetClass::Vertice => tr("vértice").to_string(),
    }
}

/// Cria a anotação com as seleções atuais; sem seleções suficientes, espera por elas.
pub fn command(app: &mut App, i: u8) {
    if i == 10 {
        auto_balloons(app);
        return;
    }
    if i == 21 || i == 22 {
        // Criar captura fica na família do detalhamento 3D (B368)
        crate::pmi_arvore::criar_captura(app, i == 22);
        return;
    }
    if i == 20 {
        if app.in_asm() {
            app.toast(tr("A solda dos cordões vem da peça: abra a peça com o cordão"), true);
        } else {
            crate::pmi_alvos_ui::solda_dos_cordoes(app);
        }
        return;
    }
    set_variant(None);
    let kind = kind_of(i);
    if i == crate::pmi_smart::SMART {
        // cota inteligente: o tipo sai dos elementos escolhidos (B268)
        app.pmi_pending = Some(i);
        app.toast(tr("Cota: escolha os elementos; o tipo é reconhecido pela seleção").to_string(), false);
        return;
    }
    if is_multi(i) {
        app.pmi_pending = Some(i);
        app.toast(format!("{}: {}", tr(if i == 13 { "Cadeia de cotas" } else { "Cotas a partir de uma base" }), tr("clique os pontos em ordem e conclua com Enter")), false);
        return;
    }
    // legenda 3D: não precisa de seleção; o diálogo deixa escolher o canto ou o ponto onde ela nasce (B299)
    if let PmiKind::TitleBlock { .. } = kind {
        let (mut lo, mut hi): (Option<[f64; 3]>, Option<[f64; 3]>) = (None, None);
        if let Some(r) = app.result.as_ref() {
            for b in &r.bodies {
                for v in b.mesh.positions.chunks_exact(3) {
                    let v = [v[0] as f64, v[1] as f64, v[2] as f64];
                    let (l, h) = (lo.get_or_insert(v), hi.get_or_insert(v));
                    for k in 0..3 {
                        l[k] = l[k].min(v[k]);
                        h[k] = h[k].max(v[k]);
                    }
                }
            }
        }
        let ponto = app.picks.iter().find_map(|p| match *p {
            Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
            _ => None,
        });
        crate::pmi_baloes_ui::abrir_legenda(lo.unwrap_or([0.; 3]), hi.unwrap_or([0.; 3]), ponto);
        return;
    }
    let picks = app.picks.clone();
    match anchors(app, &kind, &picks) {
        Some((a, t)) => create(app, kind, a, t, &picks),
        None => {
            app.pmi_pending = Some(i);
            let need = match &kind {
                PmiKind::Radius | PmiKind::Diameter => tr("uma aresta circular ou face cilíndrica").to_string(),
                PmiKind::Gap { .. } => tr("dois furos ou cilindros").to_string(),
                k => format!("{} {}", k.anchors(), tr("elemento(s): vértice, aresta ou face")),
            };
            app.toast(format!("{}: {} {need}", tr(kind.label()), tr("escolha")), false);
        }
    }
}

/// Legenda 3D nascida no ponto escolhido no diálogo, com um passo de desfazer (B299).
pub fn criar_legenda(app: &mut App, ponto: [f64; 3], off: [f64; 3]) {
    if app.in_asm() {
        app.asm.as_mut().unwrap().push_undo();
    } else {
        let antes = app.open.doc.clone();
        app.push_undo_pub(antes);
    }
    create(app, PmiKind::TitleBlock { template: TITLE_TEMPLATE.into() }, vec![ponto], vec![], &[]);
    if let Some(p) = list_mut(app).last_mut() {
        p.offset = off;
    }
}

fn create(app: &mut App, kind: PmiKind, a: Vec<[f64; 3]>, targets: Vec<Option<PmiTarget>>, picks: &[Pick]) {
    set_variant(None);
    // chamada de feature: amarra a feature dona da face e grava o texto gerado
    let (kind, gen_text) = match (&kind, picks.first()) {
        (PmiKind::FeatureNote { .. }, Some(p @ Pick::Face { .. })) => {
            let internal = targets.first().copied().flatten().is_none_or(|t| t.concave);
            let (comp, feature) = crate::pmi_feat_asm::dono_do_pick(app, p).unwrap_or((Id(0), Id(0)));
            let text = crate::pmi_feat_asm::chamada_de(app, comp, feature, internal).map(|c| c.text).unwrap_or_default();
            (PmiKind::FeatureNote { feature, comp }, text)
        }
        _ => (kind, String::new()),
    };
    // alvo novo: pertence à última referência criada e recebe o próximo número dela
    let kind = match kind {
        PmiKind::DatumTarget { mut alvo } => {
            alvo.letra = list(app).iter().rev().find_map(|p| if let PmiKind::Datum { letter } = &p.kind { Some(letter.clone()) } else { None }).unwrap_or_else(|| "A".into());
            alvo.numero = fenkra_core::pmi_alvos::proximo_numero(list(app), &alvo.letra);
            PmiKind::DatumTarget { alvo }
        }
        k => k,
    };
    // referência nova recebe a próxima letra livre (nunca I, O ou Q)
    let kind = match kind {
        PmiKind::Datum { .. } => PmiKind::Datum { letter: crate::pmi_gdt_ui::proxima(list(app)) },
        k => k,
    };
    // balão à mão: nasce com o próximo número livre e marcado para os automáticos não o trocarem
    let kind = match kind {
        PmiKind::Balloon { .. } => PmiKind::Balloon { item: fenkra_core::pmi_baloes::proximo_item(list(app)), manual: true },
        k => k,
    };
    let plane = chosen_plane(app, picks, &a);
    let id = Id(app.next_pmi_id());
    let n = list(app).iter().filter(|p| p.kind.label() == kind.label()).count() + 1;
    let name = format!("{} {n}", tr(kind.label()));
    let off = match kind {
        ref k if k.is_linear_like() => [0., 0., 12.],
        PmiKind::Angle | PmiKind::ArcLength => [0., 0., 12.],
        PmiKind::TitleBlock { .. } => [20., 0., 20.],
        _ => [15., 0., 15.],
    };
    list_mut(app).push(Pmi { id, name, kind, anchors: a, offset: off, text: gen_text, tol_plus: String::new(), tol_minus: String::new(), visible: true, plane, style: Default::default(), folder: String::new(), targets });
    app.picks.clear();
    app.pmi_pending = None;
    if app.in_asm() {
        let st = app.asm.as_mut().unwrap();
        st.sel = Some(id);
        st.modified = true;
    } else {
        app.sel = Sel::Pmi(id);
    }
    // a mini barra abre sobre a anotação nova (B270)
    crate::pmi_minibar::open_for(app, id);
}

/// Balões 3D automáticos na montagem: um por componente (número de item fixo da lista de peças),
/// presos no alto de cada componente e afastados para fora do conjunto. Refaz os anteriores.
fn auto_balloons(app: &mut App) {
    if !app.in_asm() {
        app.toast(tr("Balões automáticos: abra uma montagem na bancada Montagem"), true);
        return;
    }
    let eye = app.viewport.as_ref().map(|v| v.camera.eye());
    let st = app.asm.as_mut().unwrap();
    let dir = st.dir();
    let rows = fenkra_montagem::bom::bom(&st.asm, dir.as_deref(), fenkra_montagem::bom::BomLevel::Topo);
    let nodes: Vec<(Id, String, String)> = st
        .asm
        .nodes
        .iter()
        .filter(|n| n.visible)
        .filter_map(|n| match &n.kind {
            fenkra_montagem::NodeKind::Part { path } | fenkra_montagem::NodeKind::SubAssembly { path } => Some((n.id, path.clone(), st.asm.config_de(n.id).to_string())),
            _ => None,
        })
        .collect();
    let mut seen: Vec<String> = vec![];
    let mut found: Vec<([f64; 3], String, f64)> = vec![];
    for (id, path, cfg) in nodes {
        // B102: a mesma peça em duas configurações é dois itens, com dois balões
        let chave = if cfg.is_empty() { path.clone() } else { format!("{path}#{cfg}") };
        if seen.contains(&chave) {
            continue;
        }
        seen.push(chave);
        let Some(sh) = fenkra_montagem::view::placed_shape(&st.asm, id, dir.as_deref(), &mut st.cache) else { continue };
        let Some((l, h)) = sh.bbox() else { continue };
        let fp = fenkra_montagem::load::full_path(dir.as_deref(), &path);
        let item = rows.iter().find(|r| r.path == fp && r.config == cfg).map(|r| r.item.clone()).unwrap_or_else(|| seen.len().to_string());
        let size = ((h[0] - l[0]).powi(2) + (h[1] - l[1]).powi(2)).sqrt();
        // âncora numa face visível: o triângulo mais voltado para a câmera (e maior), não o CG
        let mut anchor = [(l[0] + h[0]) / 2., (l[1] + h[1]) / 2., h[2]];
        if let (Some(e), Ok(m)) = (eye, sh.mesh(fenkra_core::qualidade::desvio_com_fator(size.max(1.), 0.01))) {
            let p = |i: u32| Vec3::new(m.positions[i as usize * 3], m.positions[i as usize * 3 + 1], m.positions[i as usize * 3 + 2]);
            let mut best = (0f32, None);
            for t in m.indices.chunks_exact(3) {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                let n = (b - a).cross(c - a);
                let area = n.length() / 2.;
                if area < 1e-9 {
                    continue;
                }
                let ctr = (a + b + c) / 3.;
                let facing = (n / (2. * area)).dot((e - ctr).normalize_or_zero());
                if facing > 0.3 {
                    let score = area.sqrt() * facing * facing;
                    if score > best.0 {
                        best = (score, Some(ctr));
                    }
                }
            }
            if let (_, Some(c)) = best {
                anchor = [c.x as f64, c.y as f64, c.z as f64];
            }
        }
        found.push((anchor, item, size));
    }
    if found.is_empty() {
        app.toast(tr("A montagem não tem componentes visíveis"), true);
        return;
    }
    // nada muda ainda: o diálogo mostra a prévia e as contagens, e só a confirmação aplica (B299)
    let cands = found.into_iter().map(|(ancora, item, tamanho)| fenkra_core::pmi_baloes::Candidato { item, ancora, tamanho }).collect();
    crate::pmi_baloes_ui::abrir_baloes(cands);
}

/// Seleções feitas com uma anotação pendente.
pub fn after_viewport(app: &mut App) {
    let Some(i) = app.pmi_pending else { return };
    if is_multi(i) {
        return;
    }
    // chamada de feature na montagem: a peça do corpo escolhido precisa estar no cache (B186)
    if let (15, true, Some(Pick::Face { body, .. })) = (i, app.in_asm(), app.picks.last().copied()) {
        crate::pmi_feat_asm::aquecer(app, body);
    }
    if i == crate::pmi_smart::SMART {
        smart_step(app, false);
        return;
    }
    let kind = kind_now(i);
    let picks = app.picks.clone();
    if kind == PmiKind::Angle && picks.len() >= 3 {
        // ordem inválida: diz qual clique é o vértice e descarta o último ponto
        let a = anchors(app, &kind, &picks).map(|x| x.0);
        if let Some(Err(rz)) = a.as_ref().filter(|a| a.len() >= 3).map(|a| fenkra_core::pmi_infer::check_angle(a[0], a[1], a[2])) {
            app.toast(format!("{} ({}: 1º {})", tr(rz.message()), tr("vértice"), tr("clique")), false);
            app.picks.pop();
            return;
        }
    }
    if i == 16 {
        // nova(s) ponta(s) numa nota que já existe: clique simples põe uma; com Ctrl ou Shift
        // as escolhas se somam e o Enter conclui (B368)
        if !app.picks.is_empty() && !crate::pmi_setas::modificador() {
            aplicar_pontas(app, None);
        }
        return;
    }
    if let Some((a, t)) = anchors(app, &kind, &picks) {
        create(app, kind, a, t, &picks);
    }
}

/// Passo da cota inteligente depois do clique: cria quando a seleção basta (ou quando `confirm`
/// aceita a cota provisória) e recusa o clique que torna a seleção inválida.
fn smart_step(app: &mut App, confirm: bool) {
    let Some(r) = model(app) else { return };
    let picks = app.picks.clone();
    let inf = fenkra_core::pmi_infer::infer(&crate::pmi_smart::items(r, &picks));
    use fenkra_core::pmi_infer::Verdict;
    match inf.verdict {
        Verdict::Invalid(rz) => {
            app.toast(tr(rz.message()).to_string(), false);
            app.picks.pop();
        }
        Verdict::Complete | Verdict::Provisional if inf.verdict == Verdict::Complete || confirm => {
            let kind = crate::pmi_smart::kind_for(&inf);
            if let Some((a, t)) = anchors(app, &kind, &picks) {
                create(app, kind, a, t, &picks);
            }
        }
        _ => {}
    }
}

/// Furos que a geometria tem com o diâmetro dado entre as faces geradas pela feature `owner`
/// (um por eixo, mesmo que o kernel parta o cilindro em duas faces).
fn holes_found(r: &RecomputeResult, owner: Id, dia: f64) -> usize {
    let mut eixos: Vec<[i64; 3]> = vec![];
    for b in &r.bodies {
        for (f, o) in b.face_owner.iter().enumerate() {
            if *o != owner {
                continue;
            }
            let Some(t) = face_target(&b.mesh, f).filter(|t| t.class == TargetClass::FaceCilindrica && t.concave && (t.radius * 2. - dia).abs() < 1e-3) else { continue };
            let l = (t.dir[0] * t.dir[0] + t.dir[1] * t.dir[1] + t.dir[2] * t.dir[2]).sqrt().max(1e-12);
            let n = [t.dir[0] / l, t.dir[1] / l, t.dir[2] / l];
            let k = t.point[0] * n[0] + t.point[1] * n[1] + t.point[2] * n[2];
            // posição do eixo no plano perpendicular a ele, em centésimos de mm
            let key = [0, 1, 2].map(|i| ((t.point[i] - n[i] * k) * 100.).round() as i64);
            if !eixos.contains(&key) {
                eixos.push(key);
            }
        }
    }
    eixos.len()
}

/// Quebra a chamada de feature em cotas simples: o diâmetro vira uma cota de diâmetro viva, com
/// tolerância própria; contagem, profundidade, rosca, rebaixo e escareado viram notas com o texto
/// de cada parte (forma que o AP242 exige). As novas ficam numa pasta e a chamada composta sai.
fn split_feature(app: &mut App, id: Id) {
    let Some(pm) = list(app).iter().find(|p| p.id == id).cloned() else { return };
    let PmiKind::FeatureNote { feature, comp } = pm.kind else { return };
    let Some(&a0) = pm.anchors.first() else { return };
    let tgt = pm.targets.first().copied().flatten();
    let internal = tgt.is_none_or(|t| t.concave);
    let Some(c) = crate::pmi_feat_asm::chamada_de(app, comp, feature, internal) else { return };
    let folder = format!("{} ({})", pm.name, tr("cotas simples"));
    let mut novas: Vec<Pmi> = vec![];
    for (k, part) in c.parts.iter().enumerate() {
        let off = [pm.offset[0], pm.offset[1], pm.offset[2] - 6. * k as f64];
        let cilindro = tgt.filter(|t| t.class == TargetClass::FaceCilindrica);
        let (kind, anchors, targets, text) = match (part.label, cilindro) {
            ("diâmetro", Some(t)) => (PmiKind::Diameter, vec![t.carry(&t, a0, true), t.carry(&t, a0, false)], vec![Some(t), None], String::new()),
            _ => (PmiKind::Note, vec![a0], vec![], part.text.clone()),
        };
        novas.push(Pmi { id: Id(0), name: format!("{} {}", tr(kind.label()), tr(part.label)), kind, anchors, offset: off, text, tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: pm.plane, style: pm.style.clone(), folder: folder.clone(), targets });
    }
    for n in novas.iter_mut() {
        n.id = Id(app.next_pmi_id());
    }
    let l = list_mut(app);
    l.retain(|p| p.id != id);
    l.extend(novas);
    if app.sel == Sel::Pmi(id) {
        app.sel = Sel::None;
    }
}

/// Conclui a cadeia ou a base: cada elemento escolhido vira um ponto e o conjunto gera uma cota
/// por trecho, todas numa pasta da árvore (apagam e movem juntas).
/// Pontos e alvos de cada elemento escolhido (vértice, aresta ou face), para virarem setas.
fn pontas_das_escolhas(app: &App) -> Vec<([f64; 3], Option<PmiTarget>)> {
    app.picks
        .iter()
        .filter_map(|p| {
            let pos = match *p {
                Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
                Pick::Edge { mid, .. } => mid.map(|v| v as f64),
                Pick::Face { point, .. } => point.map(|v| v as f64),
                _ => return None,
            };
            Some((pos, app.result.as_ref().and_then(|r| pick_target(r, p))))
        })
        .collect()
}

/// Todas as faces, arestas ou vértices escolhidos (Ctrl e Shift somam) viram setas da nota. Com
/// `trocar`, a primeira escolha substitui a seta `k` e as outras entram como setas novas, com a
/// mesma ancoragem e ponta (B368).
fn aplicar_pontas(app: &mut App, trocar: Option<(Id, usize)>) {
    let pts = pontas_das_escolhas(app);
    if pts.is_empty() {
        app.toast(tr("Escolha ao menos uma face, aresta ou vértice").to_string(), false);
        return;
    }
    let alvo = match trocar {
        Some((id, _)) => Some(id),
        None => PONTA_PARA.lock().unwrap_or_else(|e| e.into_inner()).take(),
    };
    if let Some(id) = alvo {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            let mut resto = pts.as_slice();
            if let Some((_, k)) = trocar {
                if k < pm.anchors.len() {
                    pm.targets.resize(pm.anchors.len(), None);
                    pm.anchors[k] = pts[0].0;
                    pm.targets[k] = pts[0].1;
                    resto = &pts[1..];
                }
            }
            let (lado, ponta) = trocar.map_or((0, PmiArrow::Padrao), |(_, k)| (fenkra_core::pmi_leader::side(pm, k), fenkra_core::pmi_leader::arrow(pm, k)));
            for (p, t) in resto {
                fenkra_core::pmi_leader::add_tip(pm, *p, *t);
                let k = pm.anchors.len() - 1;
                fenkra_core::pmi_leader::set_side(pm, k, lado);
                fenkra_core::pmi_leader::set_arrow(pm, k, ponta);
            }
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    app.picks.clear();
    if trocar.is_none() {
        app.pmi_pending = None;
    }
}

fn finish_multi(app: &mut App) {
    let Some(i) = app.pmi_pending else { return };
    if i == 16 {
        aplicar_pontas(app, None);
        return;
    }
    let kind = kind_now(i);
    let picks = app.picks.clone();
    let pts: Vec<([f64; 3], Option<PmiTarget>)> = picks
        .iter()
        .filter_map(|p| {
            let pos = match *p {
                Pick::Vertex { pos, .. } => pos.map(|v| v as f64),
                Pick::Edge { mid, .. } => mid.map(|v| v as f64),
                Pick::Face { point, .. } => point.map(|v| v as f64),
                _ => return None,
            };
            let t = app.result.as_ref().and_then(|r| pick_target(r, p));
            let circ = matches!(kind, PmiKind::Gap { .. });
            Some(match t {
                Some(t) if circ && matches!(t.class, TargetClass::ArestaCircular | TargetClass::FaceCilindrica) => (t.carry(&t, pos, true), Some(t)),
                t => (pos, t),
            })
        })
        .collect();
    if pts.len() < 2 {
        app.toast(tr("Escolha ao menos dois pontos").to_string(), false);
        return;
    }
    let (parts, name) = if i == 13 {
        (fenkra_core::pmi_cotas::chain(&pts, [0., 0., 12.]), tr("Cadeia"))
    } else {
        (fenkra_core::pmi_cotas::baseline(&pts, [0., 0., 1.], 10.), tr("Base"))
    };
    let n = list(app).iter().filter(|p| p.folder.starts_with(name)).map(|p| p.folder.clone()).collect::<std::collections::BTreeSet<_>>().len() + 1;
    let folder = format!("{name} {n}");
    let mut novas = fenkra_core::pmi_cotas::build(&kind, &parts, 0, &tr(kind.label()).to_string(), &folder);
    let mut last = None;
    for pm in novas.iter_mut() {
        pm.id = Id(app.next_pmi_id());
        pm.plane = chosen_plane(app, &picks, &pm.anchors);
        last = Some(pm.id);
    }
    list_mut(app).extend(novas);
    app.picks.clear();
    app.pmi_pending = None;
    set_variant(None);
    if let Some(id) = last {
        if app.in_asm() {
            let st = app.asm.as_mut().unwrap();
            st.sel = Some(id);
            st.modified = true;
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
}

/// Reencaixa a âncora no vértice ou meio de aresta mais próximo (até 2 mm).
fn snap(r: &RecomputeResult, p: [f64; 3]) -> [f64; 3] {
    let mut best = (2.0f64, p);
    for b in &r.bodies {
        for q in b.mesh.vertices.chunks_exact(3).chain(b.mesh.edge_mids.chunks_exact(3)) {
            let q = [q[0] as f64, q[1] as f64, q[2] as f64];
            let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
            if d < best.0 {
                best = (d, q);
            }
        }
    }
    best.1
}

fn arr3(v: Vec3) -> [f64; 3] {
    [v.x as f64, v.y as f64, v.z as f64]
}

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

// ---- plano de anotação da sessão ----------------------------------------------------------

/// Escolha do plano do texto, válida até fechar o programa: vira o padrão das próximas anotações.
#[derive(Clone)]
struct PlanePref {
    /// 0 = de frente para a tela; n = n-ésimo candidato da lista
    sel: usize,
    /// quantas vezes a face base foi trocada
    shift: usize,
    /// texto alinhado à geometria (direção entre as âncoras) em vez da direção do plano
    align: bool,
    /// nome do plano ativo, mostrado na barra de status
    name: String,
}

static PREF: std::sync::Mutex<PlanePref> = std::sync::Mutex::new(PlanePref { sel: 0, shift: 0, align: false, name: String::new() });

fn pref() -> PlanePref {
    PREF.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn set_pref(f: impl FnOnce(&mut PlanePref)) {
    f(&mut PREF.lock().unwrap_or_else(|e| e.into_inner()));
}

struct PlaneCand {
    name: String,
    n: [f64; 3],
    x: [f64; 3],
    origin: [f64; 3],
}

fn frame_x(n: [f64; 3]) -> [f64; 3] {
    let n = v3(n).normalize_or_zero();
    let h = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let x = h.cross(n).normalize_or_zero();
    [x.x as f64, x.y as f64, x.z as f64]
}

/// Face base do comando: a primeira face escolhida ou, sem escolha, a que está sob o cursor.
fn base_face(app: &App, picks: &[Pick]) -> Option<(usize, usize)> {
    picks.iter().chain(app.hover_pick.iter()).find_map(|p| match *p {
        Pick::Face { body, face, .. } => Some((body, face)),
        _ => None,
    })
}

/// Planos candidatos: a face base, as faces planas vizinhas dela e os planos principais.
fn plane_cands(r: &RecomputeResult, base: Option<(usize, usize)>, shift: usize, at: [f64; 3]) -> Vec<PlaneCand> {
    let mut out = vec![];
    if let Some((b, f)) = base {
        if let Some(m) = crate::picking::mesh_of(r, b) {
            let mut ring = vec![f];
            for ef in &m.edge_faces {
                for (me, other) in [(ef[0], ef[1]), (ef[1], ef[0])] {
                    let g = other as usize;
                    if me as usize == f && other != fenkra_kernel::NO_FACE && !ring.contains(&g) && m.face_info.get(g).is_some_and(|i| i.kind == 0) && ring.len() < 6 {
                        ring.push(g);
                    }
                }
            }
            if m.face_info.get(f).is_some_and(|i| i.kind == 0) {
                let k = shift % ring.len();
                ring.rotate_left(k);
                for (i, g) in ring.iter().enumerate() {
                    let fi = &m.face_info[*g];
                    out.push(PlaneCand { name: tr(if i == 0 { "face base" } else { "face vizinha" }).to_string(), n: fi.dir, x: frame_x(fi.dir), origin: fi.point });
                }
            }
        }
    }
    for (name, n, x) in [("XY", [0., 0., 1.], [1., 0., 0.]), ("XZ", [0., -1., 0.], [1., 0., 0.]), ("YZ", [1., 0., 0.], [0., 1., 0.])] {
        out.push(PlaneCand { name: format!("{} {name}", tr("Plano")), n, x, origin: at });
    }
    out
}

/// Direção do texto alinhada à geometria: a linha entre as duas primeiras âncoras, no plano.
fn align_x(n: [f64; 3], a: &[[f64; 3]]) -> Option<[f64; 3]> {
    let d = v3(*a.get(1)?) - v3(*a.first()?);
    let n = v3(n).normalize_or_zero();
    let x = (d - n * d.dot(n)).normalize_or_zero();
    (x.length() > 0.5).then(|| [x.x as f64, x.y as f64, x.z as f64])
}

/// Plano (normal, x) que a nova anotação recebe pela escolha da sessão; `None` = de frente.
fn chosen_plane(app: &App, picks: &[Pick], anchors: &[[f64; 3]]) -> Option<([f64; 3], [f64; 3])> {
    let pf = pref();
    if pf.sel == 0 {
        return None;
    }
    let c = plane_cands(app.result.as_ref()?, base_face(app, picks), pf.shift, anchors.first().copied().unwrap_or([0.; 3]));
    let c = c.get((pf.sel - 1) % c.len().max(1))?;
    let x = if pf.align { align_x(c.n, anchors).unwrap_or(c.x) } else { c.x };
    Some((c.n, x))
}

/// Cota vista de lado: a direção entre as âncoras é (quase) a normal do plano do texto.
fn plane_hides(n: [f64; 3], a: &[[f64; 3]]) -> bool {
    let (Some(p), Some(q)) = (a.first(), a.get(1)) else { return false };
    let d = v3(*q) - v3(*p);
    d.length() > 1e-6 && d.normalize().dot(v3(n).normalize_or_zero()).abs() > 0.9
}

/// Nome do plano do texto de uma anotação, igual ao mostrado na barra de status.
fn plane_name(pl: Option<([f64; 3], [f64; 3])>) -> String {
    let Some((n, _)) = pl else { return tr("de frente para a tela").to_string() };
    let n = v3(n).normalize_or_zero();
    for (name, v) in [("XY", Vec3::Z), ("XZ", -Vec3::Y), ("YZ", Vec3::X)] {
        if n.dot(v).abs() > 0.999 {
            return format!("{} {name}", tr("Plano"));
        }
    }
    tr("face / plano da seleção").to_string()
}

/// Plano de anotação escolhido, na barra de status, enquanto o comando de PMI está aberto.
pub fn status(app: &App, ui: &mut egui::Ui) {
    let pf = pref();
    if app.pmi_pending.is_some() || pf.sel > 0 {
        ui.separator();
        let nome = if pf.name.is_empty() { tr("de frente para a tela").to_string() } else { pf.name };
        ui.label(RichText::new(format!("▱ {}: {nome}", tr("plano de anotação"))).small());
    }
}

/// Caixa de dica junto ao cursor: linhas (texto, cor) sobre fundo da janela.
fn tip(p: &egui::Painter, at: Pos2, lines: &[(String, Color32)], bg: Color32) {
    let gs: Vec<_> = lines.iter().map(|(t, c)| p.layout_no_wrap(t.clone(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), *c)).collect();
    let w = gs.iter().map(|g| g.size().x).fold(0., f32::max) + 12.;
    let h = gs.iter().map(|g| g.size().y).sum::<f32>() + 8.;
    p.rect_filled(Rect::from_min_size(at, egui::vec2(w, h)), 4., bg);
    let mut y = at.y + 4.;
    for g in gs {
        let hh = g.size().y;
        p.galley(egui::pos2(at.x + 6., y), g, Color32::WHITE);
        y += hh;
    }
}

/// Símbolo de solda ISO 2553 na tela: a junção da seta fica em `at`; `h` = altura do texto (px).
fn weld_draw(p: &Cv, at: Pos2, kind: WeldKind, size: &str, all: bool, x: &WeldExtra, pen: &Pen) -> Rect {
    let h = pen.size * 1.05;
    let m = |q: [f64; 2]| at + egui::vec2(q[0] as f32, -q[1] as f32);
    let st = pen.stroke();
    let tc = if pen.sel { HL } else { pen.text };
    let mut r = Rect::from_center_size(at, egui::vec2(pen.size * 0.4, pen.size * 0.4));
    let mut shapes: Vec<egui::Shape> = vec![];
    for pr in weld_symbol(kind, size, all, x, h as f64) {
        match pr {
            WeldPrim::Line(v) | WeldPrim::Dashed(v) if v.len() < 2 => {}
            WeldPrim::Line(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.push(egui::Shape::line(pts, st));
            }
            WeldPrim::Dashed(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.extend(egui::Shape::dashed_line(&pts, st, pen.size * 0.4, pen.size * 0.25));
            }
            WeldPrim::Fill(v) => {
                let pts: Vec<Pos2> = v.iter().map(|q| m(*q)).collect();
                pts.iter().for_each(|q| r.extend_with(*q));
                shapes.push(egui::Shape::convex_polygon(pts, st.color, Stroke::NONE));
            }
            WeldPrim::Text(q, t, al) => {
                let g = p.layout_no_wrap(t, pen.font(1.), tc);
                let pos = m(q);
                let x0 = match al {
                    -1 => pos.x - g.size().x,
                    0 => pos.x - g.size().x / 2.,
                    _ => pos.x,
                };
                let tl = egui::pos2(x0, pos.y - g.size().y * 0.8);
                r = r.union(Rect::from_min_size(tl, g.size()));
                shapes.push(egui::Shape::galley(tl, g, tc));
            }
        }
    }
    let r = r.expand(pen.size * 0.3);
    match pen.bg {
        PmiBg::Branco => p.rect_filled(r, 2., Color32::from_rgba_unmultiplied(255, 255, 255, 230)),
        PmiBg::Escuro => p.rect_filled(r, 2., Color32::from_rgba_unmultiplied(40, 30, 22, 220)),
        _ => {}
    }
    p.extend(shapes);
    r
}

/// Campos da legenda 3D ({nome}, {codigo}, {descricao}, {material}, {massa}, {volume}, {data},
/// {autor}, mais as propriedades personalizadas pelo nome).
fn title_fields(app: &App) -> std::collections::HashMap<String, String> {
    let mut f = std::collections::HashMap::new();
    let mut put = |k: &str, v: String| {
        f.insert(k.to_string(), v);
    };
    put("data", fenkra_desenho::today());
    put("autor", std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default());
    let (name, props) = match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => (st.asm.name.clone(), st.asm.props.clone()),
        None => (app.open.doc.name.clone(), app.open.doc.props.clone()),
    };
    put("nome", name.clone());
    put("codigo", props.pn.clone());
    put("descricao", if props.description.is_empty() { name } else { props.description.clone() });
    if app.in_asm() {
        put("material", "—".into());
        put("massa", "—".into());
        put("volume", "—".into());
    } else {
        let vol: f64 = app.result.as_ref().map(|r| r.bodies.iter().map(|b| b.mass.volume).sum()).unwrap_or(0.);
        let dens = crate::app::MATERIALS.iter().find(|(m, _)| *m == props.material).map(|(_, d)| *d).unwrap_or(7.85);
        put("material", props.material.clone());
        put("massa", crate::massa_fmt::texto_kg(vol, dens, app.result.as_ref().is_some_and(|r| !r.bodies.is_empty())));
        put("volume", format!("{:.1} cm³", vol / 1000.).replace('.', ","));
    }
    for (k, v) in &props.custom {
        put(&k.to_lowercase(), v.clone());
    }
    f
}

/// Realça o que a anotação marca: vértice (anel), aresta ou face sob cada âncora.
fn highlight(p: &egui::Painter, r: &RecomputeResult, cam: &Camera, rect: Rect, pts: &[[f64; 3]]) {
    let d = |v: &[f32], q: &[f64; 3]| ((v[0] as f64 - q[0]).powi(2) + (v[1] as f64 - q[1]).powi(2) + (v[2] as f64 - q[2]).powi(2)).sqrt();
    for q in pts {
        let mut done = false;
        for (bi, b) in r.bodies.iter().enumerate() {
            let m = &b.mesh;
            if m.vertices.chunks_exact(3).any(|v| d(v, q) < 1e-3) {
                if let Some(s) = cam.project(rect, v3(*q)) {
                    p.circle_stroke(s, 8., Stroke::new(2.5, HL));
                }
                done = true;
                break;
            }
            if let Some(e) = m.edge_mids.chunks_exact(3).position(|v| d(v, q) < 1e-3) {
                crate::picking::draw(p, r, cam, rect, &Pick::Edge { body: bi, edge: e, mid: q.map(|x| x as f32) }, true);
                done = true;
                break;
            }
        }
        if !done {
            for bi in 0..r.bodies.len() {
                if let Some(f) = crate::picking::face_at(r, bi, *q) {
                    crate::picking::draw(p, r, cam, rect, &Pick::Face { body: bi, face: f, owner: Id(0), point: q.map(|x| x as f32), normal: [0., 0., 1.] }, true);
                    break;
                }
            }
        }
    }
}

/// Desenha o rótulo (e a chamada até `s0`) de uma anotação: `st` é o ponto do rótulo e `s1` a
/// segunda âncora (borda no raio/diâmetro). Serve para a tela e para o plano (coordenadas locais).
#[allow(clippy::too_many_arguments)]
fn draw_kind(cv: &Cv, pen: &Pen, pm: &Pmi, st: Pos2, s0: Pos2, s1: Option<Pos2>, routes: &[Vec<Pos2>], main: &str, stack: Option<&(String, String)>, fields: &std::collections::HashMap<String, String>) -> Rect {
    let (stroke, thin) = (pen.stroke(), pen.thin());
    match &pm.kind {
        PmiKind::Radius | PmiKind::Diameter => {
            let se = s1.unwrap_or(s0);
            let slot = Pen::reserve(cv);
            let lr = pen.dim_text(cv, st, main, stack);
            pen.leader(cv, slot, lr, se);
            if matches!(pm.kind, PmiKind::Diameter) {
                // diâmetro: a linha atravessa o centro até o outro lado
                let other = s0 + (s0 - se);
                cv.line_segment([se, other], thin);
                pen.arrow(cv, other, s0);
            }
            lr
        }
        PmiKind::Weld { kind, size, all_around, extra } => {
            // seta até a junta; linha de referência e símbolo ISO 2553 no rótulo
            cv.line_segment([st, s0], stroke);
            pen.arrow(cv, s0, st);
            weld_draw(cv, st, *kind, size, *all_around, extra, pen)
        }
        PmiKind::Balloon { item, .. } => {
            let dir = (s0 - st).normalized();
            let rad = pen.size + pen.size * 0.27 * item.chars().count().saturating_sub(1) as f32;
            cv.line_segment([st + dir * rad, s0], stroke);
            cv.circle_filled(s0, pen.size * 0.22, if pen.sel { HL } else { pen.line });
            pen.balloon(cv, st, item).0
        }
        PmiKind::TitleBlock { template } => pen.table(cv, st, &fenkra_desenho::resolve(template, fields)),
        PmiKind::Gdt { ch, tol, datums, extra } => {
            let linhas = extra.linhas(*ch, tol, datums);
            let slot = Pen::reserve(cv);
            let fr = pen.fcf(cv, st, *ch, &linhas, &extra.chamada());
            pen.leader(cv, slot, fr, s0);
            fr
        }
        PmiKind::Datum { letter } => pen.datum(cv, s0, st, letter),
        PmiKind::Roughness { ra, process, extra } if !extra.basico() => {
            let linhas = fenkra_core::pmi_acabamento::linhas(ra, process, extra);
            let slot = Pen::reserve(cv);
            let (rr, tip) = pen.roughness_ext(cv, st, &linhas, extra);
            cv.set(slot, egui::Shape::line_segment([tip, s0], stroke));
            pen.arrow(cv, s0, tip);
            rr
        }
        PmiKind::DatumTarget { alvo } => pen.datum_target(cv, st, s0, s1.filter(|_| alvo.movel.is_some()), alvo),
        PmiKind::Roughness { ra, process, .. } => {
            let txt = if process.trim().is_empty() { format!("Ra {ra}") } else { format!("{}\nRa {ra}", process.trim()) };
            let slot = Pen::reserve(cv);
            let (rr, tip) = pen.roughness(cv, st, &txt);
            // a chamada sai do vértice do símbolo (ele encosta na superfície)
            cv.set(slot, egui::Shape::line_segment([tip, s0], stroke));
            pen.arrow(cv, s0, tip);
            rr
        }
        _ => {
            // nota: chamada com seta até a peça e patamar no texto
            let txt = format!("{}{}{}", pm.style.prefix, pm.text, pm.style.suffix);
            // uma chamada por ponta; cada uma pode passar por vértices intermediários (B23)
            let slots: Vec<usize> = (0..routes.len().max(1)).map(|_| Pen::reserve(cv)).collect();
            let lr = pen.text_block(cv, st, &txt, PmiFrame::Nenhuma, PmiBg::Nenhum);
            // cada seta tem a sua ancoragem e a sua ponta (B368); sem escolha, o lado é automático
            if routes.is_empty() {
                pen.leader_route_ex(cv, slots[0], lr, &[s0], fenkra_core::pmi_leader::side(pm, 0), fenkra_core::pmi_leader::arrow(pm, 0));
            } else {
                for (k, (slot, rt)) in slots.iter().zip(routes).enumerate() {
                    pen.leader_route_ex(cv, *slot, lr, rt, fenkra_core::pmi_leader::side(pm, k), fenkra_core::pmi_leader::arrow(pm, k));
                }
            }
            lr
        }
    }
}

/// Desenha as anotações sobre a viewport (o clique num rótulo seleciona a anotação).
/// O alvo serve para o comando? Raio e diâmetro pedem aresta circular ou face cilíndrica.
fn target_fits(kind: &PmiKind, t: &PmiTarget) -> bool {
    match kind {
        PmiKind::Radius | PmiKind::Diameter | PmiKind::Gap { .. } => matches!(t.class, TargetClass::ArestaCircular | TargetClass::FaceCilindrica),
        _ => true,
    }
}

/// Estados do comando de anotação aberto: o cursor realça e nomeia o alvo, a anotação fantasma
/// segue o cursor com o valor, o plano de anotação aparece como moldura e as teclas valem
/// (Esc cancela sem criar nada, Backspace retira o último elemento, Espaço/F/Tab escolhem o plano).
/// Devolve o fantasma, desenhado junto com as anotações.
fn pending_preview(app: &App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, r: &RecomputeResult) -> (Option<Pmi>, bool, bool, bool) {
    let Some(i) = app.pmi_pending else { return (None, false, false, false) };
    // Tab alterna a variante de medida (linear/ao longo de X, Y, Z/entre círculos; raio/diâmetro)
    let typing = ui.ctx().egui_wants_keyboard_input();
    let painter = ui.painter_at(rect);
    let hover_pos = ui.input(|s| s.pointer.hover_pos()).filter(|q| rect.contains(*q));
    let hover = app.hover_pick.filter(|_| hover_pos.is_some());
    let hover_t = hover.as_ref().and_then(|h| pick_target(r, h));
    // cota inteligente (B268): infere o tipo da seleção mais o elemento sob o cursor
    let smart = i == crate::pmi_smart::SMART;
    let smart_inf = smart.then(|| {
        let mut its = crate::pmi_smart::items(r, &app.picks);
        let base = fenkra_core::pmi_infer::infer(&its);
        match hover.filter(|h| !app.picks.contains(h)).and_then(|h| crate::pmi_smart::item_of(r, &h)) {
            Some(h) => {
                its.push(h);
                fenkra_core::pmi_infer::infer(&its)
            }
            None => base,
        }
    });
    let has_var = match &smart_inf {
        Some(inf) => inf.variants.len() > 1,
        None => kind_now(i).has_variants(),
    };
    if !typing && has_var && ui.input_mut(|s| s.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
        if smart {
            crate::pmi_smart::tab_next();
        } else {
            set_variant(Some((i, kind_now(i).next_variant())));
        }
    }
    let kind = match &smart_inf {
        Some(inf) => crate::pmi_smart::kind_for(inf),
        None => kind_now(i),
    };
    let need = if smart { app.picks.len() + 1 } else { kind.anchors() };
    // lista fixa de elementos já escolhidos + o que está sob o cursor; na cadeia o fantasma parte
    // do último ponto e na base, da origem
    let mut ps: Vec<Pick> = if is_multi(i) {
        (if i == 13 { app.picks.last() } else { app.picks.first() }).into_iter().cloned().collect()
    } else {
        app.picks.clone()
    };
    let fits = match &smart_inf {
        Some(inf) => hover_t.is_some() && !inf.is_invalid(),
        None => hover_t.as_ref().is_some_and(|t| target_fits(&kind, t)),
    };
    if let (Some(h), true) = (hover, fits) {
        if !ps.contains(&h) {
            ps.push(h);
        }
    }
    let at: Option<[f64; 3]> = ps.first().and_then(|p| match *p {
        Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
        Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
        Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
        _ => None,
    });
    // plano de anotação: candidatos e teclas
    let base = base_face(app, &app.picks);
    let cands = plane_cands(r, base, pref().shift, at.unwrap_or([0.; 3]));
    let (mut cancel, mut pop, mut finish) = (false, false, false);
    if !typing {
        use egui::{Key, Modifiers};
        let (space, f, tab, esc, back, enter) = ui.input_mut(|s| {
            // com variantes o Tab já foi para a medida: o texto alinhado passa para a tecla T
            let align_key = if has_var { Key::T } else { Key::Tab };
            (s.consume_key(Modifiers::NONE, Key::Space), s.consume_key(Modifiers::NONE, Key::F), s.consume_key(Modifiers::NONE, align_key), s.consume_key(Modifiers::NONE, Key::Escape), s.consume_key(Modifiers::NONE, Key::Backspace), s.consume_key(Modifiers::NONE, Key::Enter))
        });
        finish = enter && (is_multi(i) || smart || i == 16);
        if space {
            set_pref(|p| p.sel = (p.sel + 1) % (cands.len() + 1));
        }
        if f {
            set_pref(|p| p.shift += 1);
        }
        if tab {
            set_pref(|p| p.align = !p.align);
        }
        cancel = esc;
        pop = back;
    }
    let pf = pref();
    let cur = (pf.sel > 0).then(|| cands.get((pf.sel - 1) % cands.len().max(1))).flatten();
    let name = cur.map(|c| c.name.clone()).unwrap_or_else(|| tr("de frente para a tela").to_string());
    set_pref(|p| p.name = name.clone());
    // moldura translúcida do plano ativo, centrada no ponto de trabalho
    if let (Some(c), Some(at)) = (cur, at.or_else(|| hover.and_then(|h| match h {
        Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
        Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
        Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
        _ => None,
    }))) {
        let n = v3(c.n).normalize_or_zero();
        let o = v3(at) - n * (v3(at) - v3(c.origin)).dot(n);
        let x = v3(c.x).normalize_or_zero();
        let y = n.cross(x);
        let s = (crate::picking::plane_size(r) * 0.35).max(8.);
        let pts: Vec<Pos2> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].iter().filter_map(|(u, v)| cam.project(rect, o + x * (u * s) + y * (v * s))).collect();
        if pts.len() == 4 {
            painter.add(egui::Shape::convex_polygon(pts, HL.gamma_multiply(0.14), Stroke::new(1.5, HL)));
        }
    }
    // fantasma: a anotação com o elemento sob o cursor como último ponto
    let ghost = anchors(app, &kind, &ps).map(|(a, t)| {
        let off = match kind {
            ref k if k.is_linear_like() => [0., 0., 12.],
            PmiKind::Angle | PmiKind::ArcLength => [0., 0., 12.],
            _ => [15., 0., 15.],
        };
        let plane = chosen_plane(app, &ps, &a);
        let (kind, text) = match (&kind, ps.first()) {
            (PmiKind::FeatureNote { .. }, Some(p @ Pick::Face { .. })) => {
                let internal = t.first().copied().flatten().is_none_or(|x| x.concave);
                let (comp, feature) = crate::pmi_feat_asm::dono_do_pick(app, p).unwrap_or((Id(0), Id(0)));
                (PmiKind::FeatureNote { feature, comp }, crate::pmi_feat_asm::chamada_de(app, comp, feature, internal).map(|c| c.text).unwrap_or_default())
            }
            _ => (kind.clone(), String::new()),
        };
        Pmi { id: Id(u64::MAX), name: String::new(), kind, anchors: a, offset: off, text, tol_plus: String::new(), tol_minus: String::new(), visible: true, plane, style: Default::default(), folder: String::new(), targets: t }
    });
    // texto junto ao cursor: passo do comando, o que foi reconhecido e o plano
    if let Some(q) = hover_pos {
        let step = app.picks.len().min(need.saturating_sub(1)) + 1;
        let mut lines = if let Some(inf) = &smart_inf {
            let mut l = vec![(format!("{} — {} {}", tr("Cota"), tr("elementos"), app.picks.len()), Color32::WHITE)];
            l.extend(crate::pmi_smart::cursor_lines(inf, app.picks.len()));
            l
        } else if is_multi(i) {
            vec![(format!("{} — {} {}  ·  {}", tr(if i == 13 { "Cadeia de cotas" } else { "Cotas a partir de uma base" }), tr("pontos"), app.picks.len(), tr("Enter conclui")), Color32::WHITE)]
        } else {
            vec![(format!("{} — {} {step}/{need}", tr(kind.label()), tr("passo")), Color32::WHITE)]
        };
        if !smart && kind.has_variants() {
            lines.push((format!("{}: {}  ·  {}", tr("Medida"), tr(&kind.variant_label()), tr("Tab alterna")), crate::theme::st_warn()));
        }
        if !smart && kind == PmiKind::Angle {
            // vértice, ponto 1 e ponto 2 no cursor; ordem inválida aponta o vértice
            let bad = ghost.as_ref().filter(|g| g.anchors.len() >= 3).and_then(|g| fenkra_core::pmi_infer::check_angle(g.anchors[0], g.anchors[1], g.anchors[2]).err());
            lines.extend(crate::pmi_smart::angle_step_lines(app.picks.len(), bad));
        }
        match &hover_t {
            Some(t) if fits => lines.push((target_label(t), crate::theme::st_ok())),
            Some(t) => lines.push((format!("{} ({})", tr("não serve para esta anotação"), target_label(t)), crate::theme::st_err())),
            None => lines.push((tr("aponte para uma face, aresta ou vértice").to_string(), crate::theme::st_warn())),
        }
        if let (Some(g), Some(pl)) = (&ghost, &ghost.as_ref().and_then(|g| g.plane)) {
            if g.kind.is_linear_like() && plane_hides(pl.0, &g.anchors) {
                lines.push((tr("este plano não mostra a cota").to_string(), crate::theme::st_err()));
            }
        }
        tip(&painter, q + egui::vec2(16., 18.), &lines, ui.visuals().window_fill().gamma_multiply(0.92));
    }
    painter.text(
        rect.center_bottom() + egui::vec2(0., -52.),
        egui::Align2::CENTER_BOTTOM,
        format!("{}: {}{}  ·  {}", tr("Plano"), name, if pf.align { format!(" ({})", tr("texto alinhado")) } else { String::new() }, tr(if has_var { "Espaço troca o plano · F troca a face base · Tab troca a medida · T alinha o texto · Backspace volta · Esc cancela" } else { "Espaço troca o plano · F troca a face base · Tab alinha o texto · Backspace volta · Esc cancela" })),
        egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)),
        crate::theme::st_warn(),
    );
    (ghost, cancel, pop, finish)
}

pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera) {
    let mut action: Option<(Id, u8)> = None;
    let mut via_req: Option<(Id, usize, usize, [f64; 3])> = None;
    let mut via_rm: Option<(Id, usize, usize)> = None;
    let mut via_add: Option<(Id, usize)> = None;
    let mut tip_rm: Option<(Id, usize)> = None;
    let mut drag: Option<(Id, [f64; 3])> = None;
    let mut hout = crate::pmi_handles::Out::default();
    let mut sel_lbl: Option<(Id, Rect)> = None;
    let Some(r) = model(app) else { return };
    let editing = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let focus = app.pmi_focus;
    let multi_sel: Vec<Id> = app.multi.borrow().clone();
    let mut click_mods: Option<(Id, bool, bool)> = None;
    let mut abrir_seta: Option<(Id, usize)> = None;
    crate::pmi_setas::marcar_modificador(ui);
    let painter = ui.painter_at(rect);
    crate::pmi_baloes_ui::previa(app, ui, rect, cam);
    let mut edit_req: Option<Id> = None;
    let mut anchor_req: Option<(Id, usize, [f64; 3], Option<PmiTarget>)> = None;
    let mut refresh: Vec<(Id, Vec<[f64; 3]>, Vec<Option<PmiTarget>>)> = vec![];
    let pick_filter = crate::picking::Filter { faces: true, edges: true, vertices: true, planes: false, hidden_env: false, exclude_sketch: None, fantasma: None };
    let mut clicked = None;
    let fields = title_fields(app);
    let enquadradas = fenkra_core::pmi_alvos::ids_enquadrados(list(app));
    let gstyle = app.settings.pmi_style.clone();
    let ctx = ui.ctx().clone();
    // comando de anotação aberto: hover tipado, fantasma, plano de anotação e teclas
    let (ghost, cancel, pop, finish) = pending_preview(app, ui, rect, cam, r);
    let cands = if ghost.is_some() || list(app).iter().any(|p| p.targets.iter().any(Option::is_some)) { candidates(&ctx, r) } else { Default::default() };
    let all: Vec<&Pmi> = list(app).iter().filter(|p| p.visible).chain(ghost.iter()).collect();
    for pm in all {
        let is_ghost = ghost.as_ref().is_some_and(|g| std::ptr::eq(g, pm));
        // chamada de feature: o texto é regerado dos parâmetros a cada quadro (muda o furo, muda a nota)
        let live = match (&pm.kind, is_ghost) {
            (PmiKind::FeatureNote { feature, comp }, false) => {
                let internal = pm.targets.first().copied().flatten().is_none_or(|t| t.concave);
                crate::pmi_feat_asm::chamada_de(app, *comp, *feature, internal)
            }
            _ => None,
        };
        let pm_vivo: Option<Pmi> = live.as_ref().map(|c| Pmi { text: c.text.clone(), ..pm.clone() });
        let pm: &Pmi = pm_vivo.as_ref().unwrap_or(pm);
        let diag: Option<PmiDiag> = if is_ghost {
            Some(PmiDiag { state: PmiState::Ok, failed: None, message: String::new(), resolved: pm.targets.clone() })
        } else {
            pm.targets.iter().any(Option::is_some).then(|| pm.diagnose(&cands))
        };
        // a contagem n× vem do padrão: se a geometria tem outra quantidade, a chamada fica ambígua
        let diag = match (&pm.kind, &live) {
            // na montagem a contagem não é conferida: a vista não guarda a feature de cada face
            (PmiKind::FeatureNote { feature, .. }, Some(c)) if !app.in_asm() => match (c.count, c.hole_diameter) {
                (Some(n), Some(d)) => {
                    let (state, message) = fenkra_core::pmi_feature::count_state(n, holes_found(r, *feature, d));
                    if state == PmiState::Ok { diag } else { Some(PmiDiag { state, failed: Some(0), message, resolved: diag.map(|x| x.resolved).unwrap_or_default() }) }
                }
                _ => diag,
            },
            _ => diag,
        };
        let bad = diag.as_ref().is_some_and(|d| !d.ok());
        // as âncoras acompanham o alvo reencontrado; sem alvo, reencaixam no vértice mais próximo
        let mut a: Vec<[f64; 3]> = pm
            .anchors
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let on_axis = (matches!(pm.kind, PmiKind::Radius | PmiKind::Diameter) && k == 0) || matches!(pm.kind, PmiKind::Gap { .. });
                match (pm.targets.get(k).copied().flatten(), diag.as_ref().and_then(|d| d.resolved.get(k).copied().flatten())) {
                    (Some(old), Some(new)) => old.carry(&new, *p, on_axis),
                    _ => snap(r, *p),
                }
            })
            .collect();
        // ângulo entre dois planos: o vértice e as pernas saem das faces de agora, não de pontos
        // guardados (que não são vértices do modelo e não podem ser reencaixados)
        if pm.kind == PmiKind::Angle && pm.targets.len() == 2 {
            let res: Vec<Option<PmiTarget>> = diag.as_ref().map(|d| d.resolved.clone()).unwrap_or_else(|| pm.targets.clone());
            if let (Some(Some(x)), Some(Some(y))) = (res.first(), res.get(1)) {
                if let Some(v) = fenkra_core::pmi_infer::angle_from_planes(x.point, x.dir, y.point, y.dir) {
                    a = v.to_vec();
                }
            }
        }
        fenkra_core::pmi_cotas::project_to_axis(&pm.kind, &mut a);
        if let Some(d) = &diag {
            fix_pair(&pm.kind, &mut a, &d.resolved);
            let moved = a.len() != pm.anchors.len() || a.iter().zip(&pm.anchors).any(|(x, y)| (0..3).any(|i| (x[i] - y[i]).abs() > 1e-6));
            if !is_ghost && d.ok() && (moved || d.resolved != pm.targets) && d.resolved.iter().any(Option::is_some) {
                refresh.push((pm.id, a.clone(), d.resolved.clone()));
            }
        }
        let Some(first) = a.first().copied() else { continue };
        let val = diag.as_ref().and_then(|d| pm.measure_targets(&d.resolved)).or_else(|| pm.measure(&a));
        let is_sel = editing == Some(pm.id) || focus == Some(pm.id) || multi_sel.contains(&pm.id);
        let estilo_vivo = if enquadradas.contains(&pm.id) && pm.style.frame == PmiFrame::Padrao { PmiStyle { frame: PmiFrame::Caixa, ..pm.style.clone() } } else { pm.style.clone() };
        let mut pen = Pen::new(&estilo_vivo, &gstyle, is_sel);
        if pen.apply_scale {
            // tamanho em mm no modelo: texto de `size` pt tem size·0,35 mm de altura
            let (rt, _) = cam.basis();
            if let (Some(a0), Some(a1)) = (cam.project(rect, v3(first)), cam.project(rect, v3(first) + rt)) {
                let k = (a1 - a0).length() * 0.35;
                if k.is_finite() && k > 0. {
                    pen = pen.scaled(k.clamp(0.05, 40.), k.sqrt().clamp(0.3, 4.));
                }
            }
        }
        // anotação com problema: vermelha (e com aviso escrito); fantasma: translúcida
        if bad {
            pen.line = crate::theme::st_err();
            pen.text = crate::theme::st_err();
        } else if is_ghost {
            pen.line = pen.line.gamma_multiply(0.6);
            pen.text = pen.text.gamma_multiply(0.7);
        }
        let (stroke, thin) = (pen.stroke(), pen.thin());
        let wrap =|t: String| format!("{}{}{}", pm.style.prefix, t, pm.style.suffix);
        let (main, stack) = pm.display_parts(val);
        let main = wrap(main);
        let off = v3(pm.offset);
        // a geometria que a anotação marca fica realçada enquanto ela está selecionada
        if (is_sel || is_ghost) && !matches!(pm.kind, PmiKind::TitleBlock { .. }) {
            highlight(&painter, r, cam, rect, &a);
        }
        // cordão da solda: a aresta marcada engrossada até a perna da anotação (só exibição, B369)
        if let Some((perna, segs)) = crate::pmi_cordao::cordao_de(r, pm, &a) {
            crate::pmi_cordao::desenhar(&painter, cam, rect, perna, &segs, crate::theme::accent(ui.visuals()));
        }
        // deitado num plano: referencial (normal, x, y) do plano; tudo é desenhado em 0,1 mm nele
        let plane = pm.plane.and_then(|(n, x)| {
            let n = v3(n).normalize_or_zero();
            let x = (v3(x) - n * v3(x).dot(n)).normalize_or_zero();
            (n.length() > 0.5 && x.length() > 0.5).then(|| (n, x, n.cross(x)))
        });
        // desenha um rótulo deitado com origem em `o` e eixos (x, y); `draw` recebe a tela local
        let on_plane = |o: Vec3, x: Vec3, y: Vec3, draw: &dyn Fn(&Cv, &Pen) -> Rect| -> Option<Rect> {
            let cv = Cv::new(&painter);
            let ppen = pen.on_plane();
            draw(&cv, &ppen);
            cv.flush_mapped(|q| cam.project(rect, o + x * (q.x / 10.) - y * (q.y / 10.)))
        };
        let rect_lbl = match &pm.kind {
            k if k.is_linear_like() && a.len() >= 2 => {
                // linhas de chamada e linha de cota afastadas pelo deslocamento
                let (p0, p1) = (v3(a[0]), v3(a[1]));
                let (q0, q1) = (p0 + off, p1 + off);
                let (Some(s0), Some(s1), Some(t0), Some(t1)) = (cam.project(rect, p0), cam.project(rect, p1), cam.project(rect, q0), cam.project(rect, q1)) else { continue };
                // as linhas de chamada passam da linha de cota: 4 px, ou `ext_over` mm quando a alça
                // de extensão foi usada (B269)
                let over = pm.style.ext_over;
                let e3 = off.normalize_or_zero();
                let ext = |s: Pos2, t: Pos2, q: Vec3| match (over > 0. && e3.length() > 0.5, cam.project(rect, q + e3 * over as f32)) {
                    (true, Some(x)) => x,
                    _ => t + (t - s).normalized() * 4.,
                };
                painter.line_segment([s0 + (t0 - s0).normalized() * 3., ext(s0, t0, q0)], thin);
                painter.line_segment([s1 + (t1 - s1).normalized() * 3., ext(s1, t1, q1)], thin);
                painter.line_segment([t0, t1], stroke);
                let cvs = Cv::new(&painter);
                if (t1 - t0).length() > 4. {
                    pen.arrow(&cvs, t0, t1);
                    pen.arrow(&cvs, t1, t0);
                }
                cvs.flush();
                match plane {
                    // no plano: texto alinhado com a linha de cota, do lado de fora
                    Some((n, px, _)) => {
                        let dirv = q1 - q0;
                        let mut x = (dirv - n * dirv.dot(n)).normalize_or_zero();
                        if x.length() < 0.5 {
                            x = px;
                        }
                        // lê da esquerda para a direita na vista atual
                        if let (Some(a0), Some(a1)) = (cam.project(rect, q0), cam.project(rect, q0 + x)) {
                            if a1.x < a0.x {
                                x = -x;
                            }
                        }
                        let mut y = n.cross(x);
                        if y.dot(off) < 0. {
                            y = -y;
                        }
                        let gap = pen.size * 0.35 * 0.9;
                        let fb = Rect::from_center_size(t0 + (t1 - t0) * 0.5, egui::vec2(20., 20.));
                        let along = (q1 - q0).normalize_or_zero() * pm.style.text_along as f32;
                        on_plane((q0 + q1) * 0.5 + along + y * gap, x, y, &|cv, pp| pp.dim_text(cv, egui::pos2(0., 0.), &main, stack.as_ref())).unwrap_or(fb)
                    }
                    None => {
                        let mid = match cam.project(rect, (q0 + q1) * 0.5 + (q1 - q0).normalize_or_zero() * pm.style.text_along as f32) {
                            Some(m) if pm.style.text_along != 0. => m,
                            _ => t0 + (t1 - t0) * 0.5,
                        };
                        let up = egui::vec2((t1 - t0).y, -(t1 - t0).x).normalized();
                        let up = if up.y > 0. || (up.y.abs() < 0.2 && up.x < 0.) { -up } else { up };
                        // o texto fica todo do lado de fora da linha de cota (mede antes de pôr)
                        let g = painter.layout_no_wrap(main.clone(), pen.font(1.), pen.text);
                        let extra = if stack.is_some() { pen.size * 2.2 } else { 0. };
                        let (w, h) = (g.size().x + extra + 10., g.size().y + 6.);
                        let gap = up.x.abs() * w / 2. + up.y.abs() * h / 2. + 3.;
                        let cv = Cv::new(&painter);
                        let r = pen.dim_text(&cv, mid + up * gap, &main, stack.as_ref());
                        cv.flush();
                        r
                    }
                }
            }
            PmiKind::Angle | PmiKind::ArcLength if a.len() >= 3 => {
                let (Some(sv), Some(s1), Some(s2)) = (cam.project(rect, v3(a[0])), cam.project(rect, v3(a[1])), cam.project(rect, v3(a[2]))) else { continue };
                painter.line_segment([sv, s1], thin);
                painter.line_segment([sv, s2], thin);
                let (d1, d2) = ((s1 - sv).normalized(), (s2 - sv).normalized());
                let rad = crate::pmi_handles::arc_radius_px(cam, rect, pm, &a);
                let pts: Vec<Pos2> = (0..=20).map(|k| {
                    let t = k as f32 / 20.;
                    sv + (d1 * (1. - t) + d2 * t).normalized() * rad
                }).collect();
                let cv = Cv::new(&painter);
                pen.arrow(&cv, pts[0], pts[2]);
                pen.arrow(&cv, pts[20], pts[18]);
                cv.flush();
                painter.add(egui::Shape::line(pts, stroke));
                let at = sv + (d1 + d2).normalized() * (rad + pen.size * 1.4);
                match plane {
                    Some((_, x, y)) => {
                        let w3 = v3(a[0]) + ((v3(a[1]) - v3(a[0])).normalize_or_zero() + (v3(a[2]) - v3(a[0])).normalize_or_zero()).normalize_or_zero() * 15.;
                        on_plane(w3, x, y, &|cv, pp| pp.dim_text(cv, egui::pos2(0., 0.), &main, stack.as_ref())).unwrap_or(Rect::from_center_size(at, egui::vec2(20., 20.)))
                    }
                    None => {
                        let cv = Cv::new(&painter);
                        let r = pen.dim_text(&cv, at, &main, stack.as_ref());
                        cv.flush();
                        r
                    }
                }
            }
            _ => {
                let lab = v3(first) + off;
                let anchor = v3(first);
                let second = match &pm.kind {
                    // alvo móvel: o segundo ponto é o fim do curso, na direção dada
                    PmiKind::DatumTarget { alvo } => alvo.movel.map(|m| anchor + v3(m.direcao).normalize_or_zero() * m.curso as f32),
                    _ => a.get(1).map(|q| v3(*q)),
                };
                // nota com várias pontas e/ou vértices intermediários: caminho de cada ponta no modelo
                let multi = matches!(pm.kind, PmiKind::Note | PmiKind::FeatureNote { .. }) && (a.len() > 1 || pm.style.leader_via.iter().any(|v| !v.is_empty()));
                let routes3: Vec<Vec<Vec3>> = if multi {
                    (0..a.len())
                        .map(|k| {
                            let mut v: Vec<Vec3> = pm.style.leader_via.get(k).map(|w| w.iter().map(|q| v3(*q)).collect()).unwrap_or_default();
                            v.push(v3(a[k]));
                            v
                        })
                        .collect()
                } else {
                    vec![]
                };
                match plane {
                    Some((n, x, y)) => {
                        // âncoras levadas ao plano do rótulo; fora dele, um traço liga ao ponto real
                        let flat = |p: Vec3| p - n * (p - lab).dot(n);
                        let loc = |p: Vec3| {
                            let d = flat(p) - lab;
                            egui::pos2(d.dot(x) * 10., -d.dot(y) * 10.)
                        };
                        let (s0l, s1l) = (loc(anchor), second.map(loc));
                        let routes_l: Vec<Vec<Pos2>> = routes3.iter().map(|r| r.iter().map(|q| loc(*q)).collect()).collect();
                        let draw = |cv: &Cv, pp: &Pen| draw_kind(cv, pp, pm, egui::pos2(0., 0.), s0l, s1l, &routes_l, &main, stack.as_ref(), &fields);
                        let Some(bb) = on_plane(lab, x, y, &draw) else { continue };
                        if (anchor - flat(anchor)).length() > 0.3 {
                            if let (Some(a0), Some(a1)) = (cam.project(rect, flat(anchor)), cam.project(rect, anchor)) {
                                painter.line_segment([a0, a1], thin);
                            }
                        }
                        bb
                    }
                    None => {
                        let (Some(s0), Some(st)) = (cam.project(rect, anchor), cam.project(rect, lab)) else { continue };
                        let s1 = second.and_then(|q| cam.project(rect, q));
                        let routes_s: Vec<Vec<Pos2>> = routes3.iter().filter_map(|r| r.iter().map(|q| cam.project(rect, *q)).collect::<Option<Vec<_>>>()).collect();
                        let cv = Cv::new(&painter);
                        let r = draw_kind(&cv, &pen, pm, st, s0, s1, &routes_s, &main, stack.as_ref(), &fields);
                        cv.flush();
                        r
                    }
                }
            }
        };
        // o fantasma só se mostra: não recebe clique nem alças
        if is_ghost {
            continue;
        }
        // estado com problema: explicação curta junto ao rótulo e anel na âncora que falhou
        if let Some(d) = diag.as_ref().filter(|d| !d.ok()) {
            let txt = format!("⚠ {} · {}", tr(d.state.label()), tr(&d.message));
            painter.text(rect_lbl.left_bottom() + egui::vec2(0., 3.), egui::Align2::LEFT_TOP, txt, egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), crate::theme::st_err());
            if let Some(sp) = d.failed.and_then(|k| a.get(k)).and_then(|q| cam.project(rect, v3(*q))) {
                painter.circle_stroke(sp, 10., Stroke::new(2.5, crate::theme::st_err()));
                painter.line_segment([sp + egui::vec2(-6., -6.), sp + egui::vec2(6., 6.)], Stroke::new(2., crate::theme::st_err()));
                painter.line_segment([sp + egui::vec2(-6., 6.), sp + egui::vec2(6., -6.)], Stroke::new(2., crate::theme::st_err()));
            }
        }
        // legibilidade ao girar (B23): chamada que se cruza ou texto deitado de perfil nesta vista
        if !is_ghost && diag.as_ref().is_none_or(|d| d.ok()) {
            if let Some(av) = crate::pmi_legivel::aviso(cam, rect, pm, &a) {
                painter.text(rect_lbl.left_bottom() + egui::vec2(0., 3.), egui::Align2::LEFT_TOP, format!("⚠ {}", tr(av.mensagem())), egui::FontId::proportional(crate::acessibilidade::fonte_3d(11.)), crate::theme::st_warn());
            }
        }
        // setas da nota: início visível e arrastável na borda da caixa, toque duplo na seta (B368)
        if plane.is_none() && crate::pmi_setas::tem_setas(&pm.kind) {
            crate::pmi_setas::interagir(ui, &painter, &pen, cam, rect, pm, &a, rect_lbl, is_sel && !bad, &mut hout, &mut abrir_seta);
        }
        // rótulo girado na tela: a área de clique envolve o rótulo girado
        let rect_lbl = if plane.is_none() && !matches!(pm.kind, PmiKind::Datum { .. } | PmiKind::Balloon { .. } | PmiKind::Weld { .. } | PmiKind::Roughness { .. } | PmiKind::DatumTarget { .. }) { pen.outer(rect_lbl) } else { rect_lbl };
        // anotação realçada/em edição: as âncoras viram alças (arraste e solte numa face, aresta ou vértice)
        if (is_sel || bad) && !matches!(pm.kind, PmiKind::TitleBlock { .. }) {
            for (k, q) in a.iter().enumerate() {
                let Some(sp) = cam.project(rect, v3(*q)) else { continue };
                let h = ui.interact(Rect::from_center_size(sp, egui::vec2(16., 16.)), egui::Id::unique(("pmi_ancora", pm.id.0, k)), egui::Sense::drag());
                painter.circle_stroke(sp, 6.5, Stroke::new(2., HL));
                painter.circle_filled(sp, 2.5, HL);
                if h.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                let h = h.on_hover_text(tr("arraste o ponto de ancoragem para outra face, aresta ou vértice"));
                if a.len() > 1 {
                    h.context_menu(|ui| {
                        if ui.button(tr("Remover esta ponta")).clicked() {
                            tip_rm = Some((pm.id, k));
                            ui.close();
                        }
                    });
                }
                if let (true, Some(m)) = (h.dragged(), h.interact_pointer_pos()) {
                    painter.extend(egui::Shape::dashed_line(&[sp, m], Stroke::new(1.5, HL), 5., 3.));
                    // candidato sob o cursor: realçado, com o tipo reconhecido
                    let pk = crate::picking::pick(r, &app.open.doc, cam, rect, m, &pick_filter);
                    let tg = pk.as_ref().and_then(|p| pick_target(r, p));
                    if let Some(p) = &pk {
                        crate::picking::draw(&painter, r, cam, rect, p, false);
                    }
                    painter.circle_stroke(m, 7., Stroke::new(2., if tg.is_some() { crate::theme::st_ok() } else { crate::theme::st_err() }));
                    if let Some(t) = &tg {
                        painter.text(m + egui::vec2(12., 12.), egui::Align2::LEFT_TOP, target_label(t), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), crate::theme::st_ok());
                    }
                }
                if h.drag_stopped() {
                    if let Some(m) = h.interact_pointer_pos() {
                        let pk = crate::picking::pick(r, &app.open.doc, cam, rect, m, &pick_filter);
                        let hit = pk.as_ref().and_then(|p| match *p {
                            Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
                            Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
                            Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
                            _ => None,
                        });
                        if let Some(p) = hit {
                            anchor_req = Some((pm.id, k, p, pk.as_ref().and_then(|p| pick_target(r, p))));
                        }
                    }
                }
            }
        }
        // vértices intermediários da chamada: alças arrastáveis no plano da tela; botão direito remove
        if is_sel && matches!(pm.kind, PmiKind::Note | PmiKind::FeatureNote { .. }) {
            for (k, via) in pm.style.leader_via.iter().enumerate() {
                for (i, q) in via.iter().enumerate() {
                    let Some(sp) = cam.project(rect, v3(*q)) else { continue };
                    let h = ui.interact(Rect::from_center_size(sp, egui::vec2(14., 14.)), egui::Id::unique(("pmi_via", pm.id.0, k, i)), egui::Sense::drag());
                    painter.rect_filled(Rect::from_center_size(sp, egui::vec2(8., 8.)), 1., HL);
                    if h.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                    let h = h.on_hover_text(tr("vértice da chamada: arraste para mudar o caminho (encaixa em 1 mm e 15°, Shift solta); botão direito remove"));
                    if let (true, Some(m)) = (h.dragged(), h.interact_pointer_pos()) {
                        let n = -cam.ray(rect, rect.center()).1;
                        let (o, dir) = cam.ray(rect, m);
                        let den = dir.dot(n);
                        if den.abs() > 1e-6 {
                            let hit = o + dir * ((v3(*q) - o).dot(n) / den);
                            // encaixe: 1 mm e 15° a partir do ponto anterior do caminho (Shift solta)
                            let prev = if i == 0 { arr3(v3(pm.anchors.first().copied().unwrap_or([0.; 3])) + off) } else { via[i - 1] };
                            let shift = ui.input(|s| s.modifiers.shift);
                            let sn = crate::pmi_handles::snap_via(prev, hit, n, shift);
                            painter.text(m + egui::vec2(14., -18.), egui::Align2::LEFT_BOTTOM, format!("{} mm", fenkra_core::pmi::num(((sn[0] - prev[0]).powi(2) + (sn[1] - prev[1]).powi(2) + (sn[2] - prev[2]).powi(2)).sqrt())), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), HL);
                            via_req = Some((pm.id, k, i, sn));
                        }
                    }
                    h.context_menu(|ui| {
                        if ui.button(tr("Remover vértice")).clicked() {
                            via_rm = Some((pm.id, k, i));
                            ui.close();
                        }
                    });
                }
            }
        }
        let resp = ui.interact(rect_lbl, egui::Id::unique(("pmi_rotulo", pm.id.0)), egui::Sense::click_and_drag());
        if editing == Some(pm.id) {
            sel_lbl = Some((pm.id, rect_lbl));
        }
        // alças da linha de cota, das linhas de apoio e do texto (B269): por cima do rótulo
        if is_sel && pm.kind.is_linear_like() && !matches!(diag.as_ref(), Some(d) if !d.ok()) {
            crate::pmi_handles::linear(ui, &painter, cam, rect, pm, &a, rect_lbl, &mut hout);
        } else if is_sel && matches!(pm.kind, PmiKind::Angle | PmiKind::ArcLength) {
            crate::pmi_handles::arco(ui, &painter, cam, rect, pm, &a, &mut hout);
        } else if is_sel && !matches!(pm.kind, PmiKind::TitleBlock { .. } | PmiKind::Balloon { .. }) {
            crate::pmi_handles::posicao(ui, &painter, cam, rect, pm, &a, rect_lbl, &mut hout);
        }
        // arrastar o rótulo: move o texto no plano da anotação (ou de frente para a tela); o gesto
        // inteiro vira um passo de desfazer
        if resp.drag_started() {
            hout.begin = true;
        }
        if resp.drag_stopped() {
            hout.end = true;
        }
        if resp.dragged() {
            if let Some(m) = resp.interact_pointer_pos() {
                let base = v3(first) + off;
                let n = match pm.plane {
                    Some((n, _)) => v3(n).normalize_or_zero(),
                    None => -cam.ray(rect, rect.center()).1,
                };
                let (o, dir) = cam.ray(rect, m);
                let den = dir.dot(n);
                if den.abs() > 1e-6 {
                    let t = (base - o).dot(n) / den;
                    let hit = o + dir * t;
                    let new_off = hit - v3(first);
                    drag = Some((pm.id, [new_off.x as f64, new_off.y as f64, new_off.z as f64]));
                }
            }
        }
        // 1 clique: realça a anotação e o que ela marca; 2 cliques: edita
        if resp.double_clicked() {
            edit_req = Some(pm.id);
        } else if resp.clicked() || resp.secondary_clicked() {
            clicked = Some(pm.id);
            if resp.clicked() {
                click_mods = Some((pm.id, ui.input(|i| i.modifiers.command), ui.input(|i| i.modifiers.shift)));
            }
        }
        // botão direito no rótulo: opções da anotação
        resp.context_menu(|ui| {
            ui.label(RichText::new(&pm.name).strong());
            if ui.button(tr("✏ Editar")).clicked() {
                action = Some((pm.id, 2));
                ui.close();
            }
            if ui.button(tr("Ocultar")).clicked() {
                action = Some((pm.id, 1));
                ui.close();
            }
            if ui.button(tr(if pm.plane.is_some() { "🖵 De frente para a tela" } else { "▱ Deitar no plano da vista atual" })).clicked() {
                action = Some((pm.id, 3));
                ui.close();
            }
            if matches!(pm.kind, PmiKind::Note | PmiKind::FeatureNote { .. }) {
                if ui.button(tr("Adicionar ponta…")).on_hover_text(tr("Clique noutro elemento do modelo: a nota ganha mais uma chamada")).clicked() {
                    action = Some((pm.id, 5));
                    ui.close();
                }
                for k in 0..pm.anchors.len() {
                    let t = if pm.anchors.len() > 1 { format!("{} {}", tr("Adicionar vértice na chamada"), k + 1) } else { tr("Adicionar vértice intermediário").to_string() };
                    if ui.button(t).clicked() {
                        via_add = Some((pm.id, k));
                        ui.close();
                    }
                }
            }
            if matches!(pm.kind, PmiKind::FeatureNote { .. }) && ui.button(tr("Quebrar em cotas simples")).on_hover_text(tr("Cada parte da chamada vira uma cota com tolerância própria")).clicked() {
                action = Some((pm.id, 4));
                ui.close();
            }
            ui.separator();
            if ui.button(tr("🗑 Apagar   Del")).clicked() {
                action = Some((pm.id, 0));
                ui.close();
            }
        });
        resp.on_hover_text(&pm.name);
    }
    // Esc cancela sem criar nada; Backspace retira o último elemento escolhido
    if cancel {
        app.pmi_pending = None;
        app.picks.clear();
        set_variant(None);
    } else if pop {
        app.picks.pop();
    } else if finish {
        if app.pmi_pending == Some(crate::pmi_smart::SMART) {
            smart_step(app, true);
        } else {
            finish_multi(app);
        }
    }
    // edição dos vértices e pontas da chamada (B23)
    let mut mexeu = false;
    if let Some((id, k, i, p)) = via_req {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            mexeu |= fenkra_core::pmi_leader::move_vertex(pm, k, i, p);
        }
    }
    if let Some((id, k, i)) = via_rm {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            mexeu |= fenkra_core::pmi_leader::remove_vertex(pm, k, i);
        }
    }
    if let Some((id, k)) = via_add {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            let label = [0, 1, 2].map(|j| pm.anchors.first().map(|q| q[j]).unwrap_or(0.) + pm.offset[j]);
            mexeu |= fenkra_core::pmi_leader::add_vertex_mid(pm, label, k).is_some();
        }
    }
    if let Some((id, k)) = tip_rm {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            mexeu |= fenkra_core::pmi_leader::remove_tip(pm, k);
        }
    }
    if mexeu && app.in_asm() {
        app.asm.as_mut().unwrap().modified = true;
    }
    for (id, a, t) in refresh {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            // os vértices da chamada acompanham a ponta que se moveu (componente da montagem, edição)
            for (k, novo) in a.iter().enumerate() {
                if let (Some(velho), Some(via)) = (pm.anchors.get(k), pm.style.leader_via.get_mut(k)) {
                    let d = [novo[0] - velho[0], novo[1] - velho[1], novo[2] - velho[2]];
                    if d.iter().any(|x| x.abs() > 1e-9) {
                        for q in via.iter_mut() {
                            *q = [q[0] + d[0], q[1] + d[1], q[2] + d[2]];
                        }
                    }
                }
            }
            pm.anchors = a;
            pm.targets = t;
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    if let Some((id, k, hit, tg)) = anchor_req {
        if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
            // alvo novo (a face substituta, por exemplo): a âncora vai para ele e o estado se refaz
            let p = match tg {
                Some(t) => t.carry(&t, hit, (matches!(pm.kind, PmiKind::Radius | PmiKind::Diameter) && k == 0) || matches!(pm.kind, PmiKind::Gap { .. })),
                None => hit,
            };
            if pm.targets.len() < pm.anchors.len() {
                pm.targets.resize(pm.anchors.len(), None);
            }
            if let Some(slot) = pm.targets.get_mut(k) {
                *slot = tg;
            }
            if k == 0 {
                // o rótulo fica no mesmo lugar: o deslocamento compensa a âncora nova
                let old = pm.anchors[0];
                pm.offset = [0, 1, 2].map(|i| pm.offset[i] + old[i] - p[i]);
            }
            if let Some(x) = pm.anchors.get_mut(k) {
                *x = p;
            }
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    if hout.begin {
        crate::pmi_handles::begin(app);
    }
    if !hout.reqs.is_empty() && crate::pmi_handles::apply(list_mut(app), &hout.reqs) && app.in_asm() {
        app.asm.as_mut().unwrap().modified = true;
    }
    if hout.end {
        crate::pmi_handles::end(app);
    }
    if let Some((id, off)) = drag {
        if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
            if p.offset != off {
                p.offset = off;
                crate::pmi_handles::mark();
                if app.in_asm() {
                    app.asm.as_mut().unwrap().modified = true;
                }
            }
        }
        clicked = Some(id);
    }
    // clique no vazio da vista: tira o realce
    if clicked.is_none() && edit_req.is_none() && action.is_none() && ui.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|q| rect.contains(q))) {
        app.pmi_focus = None;
        app.multi.borrow_mut().retain(|i| !list(app).iter().any(|p| p.id == *i));
    }
    if let Some((id, k)) = action {
        match k {
            0 => {
                list_mut(app).retain(|p| p.id != id);
                clicked = None;
                if app.sel == Sel::Pmi(id) {
                    app.sel = Sel::None;
                }
            }
            1 => {
                if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                    p.visible = false;
                }
            }
            4 => split_feature(app, id),
            5 => {
                // nova ponta: o próximo elemento escolhido no 3D vira mais uma chamada da nota
                *PONTA_PARA.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
                set_variant(None);
                app.picks.clear();
                app.pmi_pending = Some(16);
                app.toast(format!("{}: {}", tr("Adicionar ponta…"), tr("clique no elemento que a nova chamada deve marcar")), false);
            }
            3 => toggle_plane(app, id, cam, rect),
            _ => edit_req = Some(id),
        }
    }
    if let Some(id) = clicked {
        app.pmi_focus = Some(id);
    }
    // Ctrl soma ou tira, Shift soma, simples escolhe só esta (as mesmas regras das peças)
    if let Some((id, ctrl, shift)) = click_mods {
        crate::selecao::aplicar(&mut app.multi.borrow_mut(), Some(id), crate::selecao::Modo::das_teclas(ctrl, shift));
    }
    if let Some((id, k)) = abrir_seta {
        crate::pmi_setas::pedir_abrir(id, k);
        edit_req = Some(id);
    }
    if let Some(id) = edit_req {
        app.pmi_focus = Some(id);
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
        // clique duplo no 3D: a edição completa (a mesma da árvore), com o texto dentro dela (B368)
        if crate::pmi_minibar::current() == Some(id) {
            crate::pmi_minibar::drop_session();
        }
    }
    crate::pmi_minibar::show(app, ui, rect, cam, sel_lbl);
}

/// Alterna o texto da anotação: de frente para a tela ↔ deitado no plano da vista atual (fica
/// fixo nele ao girar).
pub(crate) fn toggle_plane(app: &mut App, id: Id, cam: &Camera, rect: Rect) {
    let (_, d) = cam.ray(rect, rect.center());
    let (_, dx) = cam.ray(rect, rect.center() + egui::vec2(10., 0.));
    let n = -d;
    let x = (dx - d).normalize_or_zero();
    if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
        p.plane = match p.plane {
            Some(_) => None,
            None => Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64])),
        };
    }
}

/// Capturas do nível atual (peça ou montagem).
pub(crate) fn capturas_mut(app: &mut App) -> &mut Vec<PmiCapture> {
    captures_mut(app)
}

pub(crate) fn oculto_agora(app: &App) -> Vec<Id> {
    hidden_now(app)
}

fn captures(app: &App) -> &Vec<PmiCapture> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => &st.asm.pmi_captures,
        None => &app.open.doc.pmi_captures,
    }
}

fn captures_mut(app: &mut App) -> &mut Vec<PmiCapture> {
    if app.in_asm() {
        &mut app.asm.as_mut().unwrap().asm.pmi_captures
    } else {
        &mut app.open.doc.pmi_captures
    }
}

/// O que está oculto no modelo agora: comandos, coletores e referências (peça) ou componentes.
fn hidden_now(app: &App) -> Vec<Id> {
    match app.asm.as_ref().filter(|_| app.in_asm()) {
        Some(st) => st.asm.nodes.iter().filter(|n| !n.visible).map(|n| n.id).collect(),
        None => {
            let d = &app.open.doc;
            let mut v: Vec<Id> = d.features.iter().filter(|f| f.hidden).map(|f| f.id).collect();
            v.extend(d.collectors.iter().filter(|c| !c.visible).map(|c| c.id));
            v.extend(d.references.iter().filter(|r| !r.visible).map(|r| r.id));
            v
        }
    }
}

/// Uma linha de anotação na árvore (com o menu de pastas). Usa a linha de recurso do app
/// (`row_label`: Ctrl/Shift, F2 e realce) e `drag_item` (arrastar e reordenar), como Geometria (B368).
#[allow(clippy::too_many_arguments)]
fn item_row(app: &App, ui: &mut egui::Ui, p: &Pmi, sel: Option<Id>, folders: &[String], select: &mut Option<Id>, action: &mut Option<(Id, u8)>, move_to: &mut Option<(Id, String)>, acts: &mut Vec<crate::app::TreeAction>, reorder: &mut Option<(Id, Id)>) {
    let txt = format!("{} {}", if p.visible { "📐" } else { "·" }, p.name);
    let rt = if p.visible { RichText::new(txt) } else { RichText::new(txt).weak() };
    crate::tree_ui::marca(ui);
    let r = app.row_label(ui, p.id, &p.name, sel == Some(p.id), rt, acts);
    let r = crate::app::drag_item(ui, r, p.id, app.renaming() != Some(p.id));
    let r = r.on_hover_text(crate::i18n::tr("1 clique: realça o que a anotação marca · 2 cliques: edita · arraste para reordenar · F2 renomeia"));
    if let Some(d) = r.dnd_release_payload::<crate::app::DragItem>() {
        if d.0 != p.id {
            *reorder = Some((d.0, p.id));
        }
    }
    if r.dnd_hover_payload::<crate::app::DragItem>().is_some_and(|d| d.0 != p.id) {
        ui.painter().hline(r.rect.x_range(), r.rect.bottom(), egui::Stroke::new(2., crate::theme::accent(ui.visuals())));
    }
    if r.double_clicked() {
        *select = Some(p.id);
    } else if r.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("pmi_foco_arvore"), p.id.0));
    }
    r.context_menu(|ui| {
        ui.label(RichText::new(&p.name).strong());
        if ui.button(tr("✏ Editar")).clicked() {
            *select = Some(p.id);
            ui.close();
        }
        if ui.button(tr("✎ Renomear   F2")).clicked() {
            acts.push(crate::app::TreeAction::StartRename(p.id));
            ui.close();
        }
        if ui.button(if p.visible { tr("Ocultar") } else { tr("Mostrar") }).clicked() {
            *action = Some((p.id, 1));
            ui.close();
        }
        ui.menu_button(tr("📁 Mover para pasta"), |ui| {
            if ui.button(tr("(raiz)")).clicked() {
                *move_to = Some((p.id, String::new()));
                ui.close();
            }
            for f in folders {
                if ui.button(f).clicked() {
                    *move_to = Some((p.id, f.clone()));
                    ui.close();
                }
            }
            if ui.button(tr("＋ Nova pasta")).clicked() {
                let mut n = folders.len() + 1;
                while folders.contains(&format!("{} {n}", tr("Pasta"))) {
                    n += 1;
                }
                *move_to = Some((p.id, format!("{} {n}", tr("Pasta"))));
                ui.close();
            }
        });
        ui.separator();
        if ui.button(tr("🗑 Apagar")).clicked() {
            *action = Some((p.id, 0));
            ui.close();
        }
    });
}

/// Quantas anotações e capturas há, para a árvore decidir o que está vazio.
pub fn contagem(app: &App) -> (usize, usize) {
    (list(app).len(), captures(app).len())
}

/// F2 renomeia a anotação ou a captura escolhida (na peça o app já cuida dos recursos).
fn tecla_f2_arvore(app: &mut App, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() || !ctx.input(|i| i.key_pressed(egui::Key::F2)) {
        return;
    }
    let pmi_ou_cap = |id: Id| list(app).iter().any(|p| p.id == id) || captures(app).iter().any(|c| c.id == id);
    let unico = { let m = app.multi.borrow(); if m.len() == 1 { Some(m[0]) } else { None } };
    let alvo = unico.filter(|i| pmi_ou_cap(*i)).or(app.pmi_focus.filter(|i| pmi_ou_cap(*i)));
    if let Some(id) = alvo {
        app.set_renaming(Some(id));
    }
}

/// Seção "PMI" na árvore (peça ou montagem): anotações (com pastas) e, à parte, as capturas.
pub fn tree_section(app: &mut App, ui: &mut egui::Ui, com_pmi: bool, com_capturas: bool) {
    let n = list(app).len();
    let mut select = None;
    let mut action: Option<(Id, u8)> = None;
    let mut move_to: Option<(Id, String)> = None;
    let mut folder_act: Option<(String, u8)> = None;
    let sel = if app.in_asm() { app.asm.as_ref().and_then(|s| s.sel) } else if let Sel::Pmi(i) = app.sel { Some(i) } else { None };
    let sel = sel.or(app.pmi_focus);
    let mut create: Option<u8> = None;
    let mut acts: Vec<crate::app::TreeAction> = vec![];
    let mut reorder: Option<(Id, Id)> = None;
    let mut reorder_cap: Option<(Id, Id)> = None;
    let mut folders: Vec<String> = list(app).iter().map(|p| p.folder.clone()).filter(|f| !f.is_empty()).collect();
    folders.sort();
    folders.dedup();
    if com_pmi {
    let h = crate::tree_ui::grupo(ui, "arvore_pmi", Some("pmi_nota"), format!("{} ({n})", tr("PMI — detalhamento 3D")), n > 0).corpo(ui, |ui| {
        for f in &folders {
            let r = crate::tree_ui::grupo(ui, ("pmi_pasta", f.as_str()), Some("pasta"), f.as_str(), true).corpo(ui, |ui| {
                for p in list(app).iter().filter(|p| p.folder == *f) {
                    item_row(app, ui, p, sel, &folders, &mut select, &mut action, &mut move_to, &mut acts, &mut reorder);
                }
            });
            r.context_menu(|ui| {
                if ui.button(tr("👁 Mostrar a pasta")).clicked() {
                    folder_act = Some((f.clone(), 2));
                    ui.close();
                }
                if ui.button(tr("Ocultar a pasta")).clicked() {
                    folder_act = Some((f.clone(), 3));
                    ui.close();
                }
                if ui.button(tr("Desfazer a pasta (anotações vão para a raiz)")).clicked() {
                    folder_act = Some((f.clone(), 0));
                    ui.close();
                }
            });
        }
        for p in list(app).iter().filter(|p| p.folder.is_empty()) {
            item_row(app, ui, p, sel, &folders, &mut select, &mut action, &mut move_to, &mut acts, &mut reorder);
        }
    });
    let h = h.on_hover_text(tr("Cotas, notas, rugosidade, solda e tolerâncias no modelo 3D (grupo Detalhamento 3D)."));
    h.context_menu(|ui| {
        ui.label(RichText::new(tr("Nova anotação 3D")).strong());
        for (i, n) in [(17u8, "◎ Cota inteligente"), (0u8, "↔ Cota linear"), (1, "R Raio"), (2, "Ø Diâmetro"), (3, "∠ Ângulo"), (4, "✎ Nota"), (5, "√ Rugosidade"), (6, "⌒ Solda"), (7, "▭ Referência (datum)"), (8, "⊞ Tolerância geométrica"), (9, "▤ Legenda 3D"), (11, "⊢ Cota ordenada"), (12, "⌒ Comprimento de arco"), (13, "⇔ Cadeia de cotas"), (14, "⇥ Cotas a partir de uma base"), (15, "ƒ Chamada de feature"), (18, "① Balão à mão"), (19, "⊕ Alvo de referência"), (20, "⌒ Solda dos cordões da bancada Estrutural")] {
            if ui.button(tr(n)).clicked() {
                create = Some(i);
                ui.close();
            }
        }
        if ui.button(tr("🎨 Estilo geral das anotações…")).clicked() {
            app.pmi_style_open = true;
            ui.close();
        }
        if app.in_asm() && ui.button(tr("① Balões automáticos")).clicked() {
            create = Some(10);
            ui.close();
        }
        ui.separator();
        if ui.button(tr("👁 Mostrar todas")).clicked() {
            action = Some((Id(0), 2));
            ui.close();
        }
        if ui.button(tr("Ocultar todas")).clicked() {
            action = Some((Id(0), 3));
            ui.close();
        }
    });
    }
    // capturas: vista (câmera e zoom) + anotações visíveis (+ visibilidade do modelo, se pedida)
    let mut cap_act: Option<(usize, u8)> = None;
    let caps: Vec<(Id, String, bool)> = captures(app).iter().map(|c| (c.id, c.name.clone(), c.with_model)).collect();
    if com_capturas {
    let hc = crate::tree_ui::grupo(ui, "arvore_capturas", Some("vista_iso"), format!("{} ({})", tr("Capturas"), caps.len()), !caps.is_empty()).corpo(ui, |ui| {
        for (i, (cid, n, wm)) in caps.iter().enumerate() {
            crate::tree_ui::marca(ui);
            let sel_cap = app.multi.borrow().contains(cid);
            let r = app.row_label(ui, *cid, n, sel_cap, RichText::new(format!("📷 {n}{}", if *wm { " ◐" } else { "" })), &mut acts);
            let r = crate::app::drag_item(ui, r, *cid, app.renaming() != Some(*cid));
            let r = r.on_hover_text(if *wm { tr("aplicar: câmera, anotações e visibilidade do modelo") } else { tr("aplicar: câmera e anotações desta captura") });
            if let Some(d) = r.dnd_release_payload::<crate::app::DragItem>() {
                if d.0 != *cid {
                    reorder_cap = Some((d.0, *cid));
                }
            }
            if r.dnd_hover_payload::<crate::app::DragItem>().is_some_and(|d| d.0 != *cid) {
                ui.painter().hline(r.rect.x_range(), r.rect.bottom(), egui::Stroke::new(2., crate::theme::accent(ui.visuals())));
            }
            // Ctrl/Shift só escolhem (como nos recursos); o clique simples também aplica a captura
            if r.clicked() && !ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                cap_act = Some((i, 0));
            }
            r.context_menu(|ui| {
                if ui.button(tr("✎ Renomear   F2")).clicked() {
                    acts.push(crate::app::TreeAction::StartRename(*cid));
                    ui.close();
                }
                if ui.button(tr("📷 Atualizar com a vista atual")).clicked() {
                    cap_act = Some((i, 1));
                    ui.close();
                }
                if ui.button(if *wm { tr("Não guardar a visibilidade do modelo") } else { tr("Guardar também a visibilidade do modelo") }).clicked() {
                    cap_act = Some((i, 4));
                    ui.close();
                }
                if ui.button(tr("🗑 Apagar captura")).clicked() {
                    cap_act = Some((i, 2));
                    ui.close();
                }
            });
        }
    });
    let hc = hc.on_hover_text(tr("Botão direito aqui: capturar a vista atual com as anotações visíveis (e, se quiser, o que está oculto no modelo)."));
    hc.context_menu(|ui| {
        if ui.button(tr("📷 Capturar: câmera + anotações")).clicked() {
            cap_act = Some((usize::MAX, 1));
            ui.close();
        }
        if ui.button(tr("📷 Capturar: câmera + anotações + visibilidade do modelo")).clicked() {
            cap_act = Some((usize::MAX, 3));
            ui.close();
        }
    });
    }
    tecla_f2_arvore(app, ui.ctx());
    for a in acts {
        match a {
            crate::app::TreeAction::StartRename(id) => app.set_renaming(Some(id)),
            crate::app::TreeAction::RenameCancel => app.set_renaming(None),
            crate::app::TreeAction::Rename(id, n) => {
                app.set_renaming(None);
                let n = n.trim().to_string();
                if !n.is_empty() {
                    if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
                        p.name = n.clone();
                    } else if let Some(c) = captures_mut(app).iter_mut().find(|c| c.id == id) {
                        c.name = n;
                    }
                    if app.in_asm() {
                        app.asm.as_mut().unwrap().modified = true;
                    }
                }
            }
            _ => {}
        }
    }
    // arrastar e soltar sobre outra linha do mesmo tipo reordena; o grupo escolhido anda junto
    for (par, cap) in [(reorder, false), (reorder_cap, true)] {
        if let Some((de, sobre)) = par {
            let grupo: Vec<Id> = {
                let m = app.multi.borrow();
                if m.len() > 1 && m.contains(&de) { m.clone() } else { vec![de] }
            };
            let feito = if cap {
                crate::pmi_arvore::reordenar(captures_mut(app), &|c| c.id, &grupo, sobre)
            } else {
                crate::pmi_arvore::reordenar(list_mut(app), &|p| p.id, &grupo, sobre)
            };
            if feito && app.in_asm() {
                app.asm.as_mut().unwrap().modified = true;
            }
        }
    }
    if let Some(f) = ui.ctx().data_mut(|d| d.remove_temp::<u64>(egui::Id::unique("pmi_foco_arvore"))) {
        app.pmi_focus = Some(Id(f));
    }
    if let Some(id) = select {
        app.pmi_focus = Some(id);
        if app.in_asm() {
            app.asm.as_mut().unwrap().sel = Some(id);
        } else {
            app.sel = Sel::Pmi(id);
        }
    }
    if let Some(i) = create {
        command(app, i);
    }
    if let Some((i, k)) = cap_act {
        let view = app.camera_get().map(|c| fenkra_core::NamedView { name: String::new(), target: c.target, yaw: c.yaw, pitch: c.pitch, dist: c.dist, ortho: c.ortho });
        let shown: Vec<Id> = list(app).iter().filter(|p| p.visible).map(|p| p.id).collect();
        let hidden = hidden_now(app);
        match k {
            0 => {
                if let Some(c) = captures(app).get(i).cloned() {
                    for p in list_mut(app) {
                        p.visible = c.shown.contains(&p.id);
                    }
                    if c.with_model {
                        if app.in_asm() {
                            let st = app.asm.as_mut().unwrap();
                            for n in &mut st.asm.nodes {
                                n.visible = !c.hidden.contains(&n.id);
                            }
                            st.modified = true;
                        } else {
                            let d = &mut app.open.doc;
                            for f in &mut d.features {
                                f.hidden = c.hidden.contains(&f.id);
                            }
                            for col in &mut d.collectors {
                                col.visible = !c.hidden.contains(&col.id);
                            }
                            for r in &mut d.references {
                                r.visible = !c.hidden.contains(&r.id);
                            }
                        }
                        app.scene_dirty = true;
                    }
                    app.apply_view(&c.view);
                }
            }
            1 | 3 => {
                if let Some(v) = view {
                    if i == usize::MAX {
                        let id = Id(app.next_pmi_id());
                        let n = captures(app).len() + 1;
                        captures_mut(app).push(PmiCapture { id, name: format!("{} {n}", tr("Captura")), view: v, shown, hidden, with_model: k == 3 });
                    } else if let Some(c) = captures_mut(app).get_mut(i) {
                        c.view = v;
                        c.shown = shown;
                        c.hidden = hidden;
                    }
                }
            }
            4 => {
                if let Some(c) = captures_mut(app).get_mut(i) {
                    c.with_model = !c.with_model;
                    c.hidden = hidden;
                }
            }
            _ => {
                if i < captures(app).len() {
                    captures_mut(app).remove(i);
                }
            }
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    if let Some((id, f)) = move_to {
        if let Some(p) = list_mut(app).iter_mut().find(|p| p.id == id) {
            p.folder = f;
        }
    }
    if let Some((f, k)) = folder_act {
        for p in list_mut(app).iter_mut().filter(|p| p.folder == f) {
            match k {
                0 => p.folder.clear(),
                _ => p.visible = k == 2,
            }
        }
    }
    if let Some((id, k)) = action {
        let l = list_mut(app);
        match k {
            0 => l.retain(|p| p.id != id),
            2 | 3 => l.iter_mut().for_each(|p| p.visible = k == 2),
            _ => {
                if let Some(p) = l.iter_mut().find(|p| p.id == id) {
                    p.visible = !p.visible;
                }
            }
        }
    }
}

/// Campos do símbolo de solda ISO 2553 (usado no PMI e no desenho).
/// Campos do quadro de controle geométrico (característica, tolerância, referências, modificador,
/// quadro composto) — usado no 3D e no desenho.
pub fn gdt_ui(ui: &mut egui::Ui, salt: &str, ch: &mut GdtChar, tol: &mut String, datums: &mut String, extra: &mut GdtExtra) {
    crate::pmi_gdt_ui::editor(ui, salt, ch, tol, datums, extra, true);
}

pub fn weld_ui(ui: &mut egui::Ui, salt: &str, kind: &mut WeldKind, size: &mut String, all_around: &mut bool, extra: &mut WeldExtra) {
    let pick = |ui: &mut egui::Ui, id: &str, k: &mut WeldKind| {
        egui::ComboBox::from_id_salt((salt, id)).selected_text(format!("{} {}", k.symbol(), tr(k.label()))).width(180.).show_ui(ui, |ui| {
            for w in WeldKind::ALL {
                ui.selectable_value(k, w, format!("{} {}", w.symbol(), tr(w.label())));
            }
        });
    };
    ui.horizontal(|ui| {
        ui.label(tr("Lado da seta"));
        pick(ui, "seta", kind);
    });
    let mut both = extra.other.is_some();
    ui.horizontal(|ui| {
        if crate::editor::check(ui, &mut both, "dos dois lados").changed() {
            extra.other = both.then_some(*kind);
        }
        if let Some(o) = &mut extra.other {
            pick(ui, "outro", o);
        } else {
            crate::editor::check(ui, &mut extra.other_side, "só do outro lado (na linha tracejada)");
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Medida"));
        ui.add(egui::TextEdit::singleline(size).desired_width(60.).hint_text("a5"));
        ui.label(tr("Comprimento"));
        ui.add(egui::TextEdit::singleline(&mut extra.length).desired_width(90.).hint_text("3×50 (100)"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("Acabamento"));
        for (c, n) in [(WeldContour::Nenhum, "nenhum"), (WeldContour::Plano, "plano"), (WeldContour::Convexo, "convexo"), (WeldContour::Concavo, "côncavo")] {
            ui.radio_value(&mut extra.contour, c, tr(n));
        }
    });
    ui.horizontal(|ui| {
        ui.checkbox(all_around, tr("todo o contorno"));
        crate::editor::check(ui, &mut extra.field, "solda de campo");
    });
    // cordão visível no 3D: linha engrossada do tamanho da perna; não muda sólido, massa nem lista de corte
    let mut mostrar = !extra.hide_bead;
    if ui.checkbox(&mut mostrar, tr("mostrar o cordão no 3D")).on_hover_text(tr("Engrossa a aresta marcada até a perna da medida (z5, a4…), como se fosse a solda. É só exibição: o sólido, a massa e a lista de corte não mudam.")).changed() {
        extra.hide_bead = !mostrar;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Cauda (processo)"));
        ui.add(egui::TextEdit::singleline(&mut extra.process).desired_width(120.).hint_text("135 / ISO 5817-C"));
    });
    ui.label(RichText::new(tr("ISO 2553, sistema A: símbolo na linha cheia = lado da seta; na tracejada = outro lado. Medida da seção à esquerda (a = garganta, z = perna), comprimento à direita, processo ISO 4063 na cauda.")).weak().small());
}

/// Painel de edição da anotação selecionada. Devolve `true` se havia uma anotação.
pub fn panel(app: &mut App, ui: &mut egui::Ui, id: Id) -> bool {
    let mut style_req = false;
    let r = panel_inner(app, ui, id, &mut style_req);
    if style_req {
        app.pmi_style_open = true;
    }
    r
}

/// Janela do estilo geral das anotações (vale para todas as que deixam "padrão").
pub fn style_window(app: &mut App, ctx: &egui::Context) {
    if !app.pmi_style_open {
        return;
    }
    let mut open = true;
    let mut done = false;
    egui::Window::new(tr("🎨 Estilo geral das anotações 3D")).open(&mut open).default_width(380.).show(ctx, |ui| {
        done = crate::app::done_bar(ui);
        ui.label(RichText::new(tr("Vale para todas as anotações que deixam o item em 'padrão'. Cada anotação pode trocar o que quiser no próprio painel.")).weak().small());
        crate::pmi_draw::style_ui(ui, &mut app.settings.pmi_style, false);
        ui.horizontal(|ui| {
            if ui.button(tr("Técnico (azul-escuro, sem fundo)")).clicked() {
                app.settings.pmi_style = PmiStyle { font: "Arial".into(), ..Default::default() };
            }
            if ui.button(tr("Alto contraste (fundo branco)")).clicked() {
                app.settings.pmi_style = PmiStyle { font: "Arial".into(), color: Some([0, 0, 0]), text_color: Some([0, 0, 0]), bg: PmiBg::Branco, ..Default::default() };
            }
            if ui.button(tr("Laranja (antigo)")).clicked() {
                app.settings.pmi_style = PmiStyle { color: Some([255, 150, 60]), text_color: Some([255, 255, 255]), bg: PmiBg::Escuro, frame: PmiFrame::Caixa, ..Default::default() };
            }
        });
    });
    if !open || done {
        app.pmi_style_open = false;
    }
}

fn panel_inner(app: &mut App, ui: &mut egui::Ui, id: Id, style_req: &mut bool) -> bool {
    let seed_pts: Vec<[f64; 3]> = app.picks.iter().filter_map(|p| match *p {
        Pick::Vertex { pos, .. } => Some(pos.map(|v| v as f64)),
        Pick::Edge { mid, .. } => Some(mid.map(|v| v as f64)),
        Pick::Face { point, .. } => Some(point.map(|v| v as f64)),
        _ => None,
    }).collect();
    // plano da face/plano clicado (para deitar o texto nele)
    let seed_plane: Option<([f64; 3], [f64; 3])> = app.picks.iter().find_map(|p| match *p {
        Pick::Face { normal, .. } => {
            let n = Vec3::new(normal[0], normal[1], normal[2]).normalize_or_zero();
            let helper = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
            let x = helper.cross(n).normalize_or_zero();
            Some(([n.x as f64, n.y as f64, n.z as f64], [x.x as f64, x.y as f64, x.z as f64]))
        }
        Pick::Plane(pr) => app.result.as_ref().and_then(|r| {
            r.planes.iter().find(|v| match pr {
                fenkra_core::PlaneRef::Datum(id) => v.id == Some(id),
                fenkra_core::PlaneRef::Origin(o) => v.id.is_none() && v.name == crate::scene::plane_name(o),
            })
        }).map(|v| (v.frame.normal, v.frame.x)),
        _ => None,
    });
    // estado da anotação frente à geometria atual e novos alvos pela seleção
    let (diag, reseed) = {
        let Some(pm) = list(app).iter().find(|p| p.id == id) else { return false };
        let diag = match (&app.result, pm.targets.iter().any(Option::is_some)) {
            (Some(r), true) => Some(pm.diagnose(&candidates(ui.ctx(), r))),
            _ => None,
        };
        (diag, anchors(app, &pm.kind, &app.picks))
    };
    crate::pmi_gdt_ui::atualizar_existentes(list(app));
    let outras = crate::pmi_gdt_ui::letras_de_outras(list(app), id);
    let todas = list(app).clone();
    let alvos_txt = list(app).iter().find(|q| q.id == id).map(|q| crate::pmi_setas::rotulos(q, &target_label)).unwrap_or_default();
    let n_escolhas = app.picks.len();
    let l = list_mut(app);
    let Some(p) = l.iter_mut().find(|p| p.id == id) else { return false };
    ui.heading(format!("📐 {}", p.name));
    ui.label(RichText::new(tr(p.kind.label())).weak());
    if let Some(d) = diag.as_ref().filter(|d| !d.ok()) {
        ui.label(RichText::new(format!("⚠ {} · {}", tr(d.state.label()), tr(&d.message))).color(crate::theme::st_err()));
        ui.label(RichText::new(tr("Arraste a âncora marcada para um elemento realçado, ou use «Reancorar na seleção atual».")).weak().small());
    } else if diag.is_some() {
        ui.label(RichText::new(format!("✔ {}", tr("OK"))).color(crate::theme::st_ok()).small());
    }
    ui.label(RichText::new(format!("{}: {}", tr("Plano do texto"), plane_name(p.plane))).small());
    if let (Some(pl), true) = (p.plane, p.kind.is_linear_like()) {
        if plane_hides(pl.0, &p.anchors) {
            ui.label(RichText::new(tr("este plano não mostra a cota")).color(crate::theme::st_err()).small());
        }
    }
    ui.horizontal(|ui| {
        ui.label(tr("Nome"));
        ui.text_edit_singleline(&mut p.name);
    });
    if p.kind.has_variants() {
        ui.horizontal(|ui| {
            ui.label(tr("Medida"));
            if ui.button(tr(&p.kind.variant_label())).on_hover_text(tr("Alterna entre as variantes de medida desta cota")).clicked() {
                p.kind = p.kind.next_variant();
            }
        });
    }
    match &mut p.kind {
        PmiKind::Note => {
            ui.label(tr("Texto"));
            ui.text_edit_multiline(&mut p.text);
        }
        PmiKind::FeatureNote { .. } => {
            ui.label(RichText::new(tr("O texto vem dos parâmetros da feature e se atualiza sozinho.")).weak().small());
            ui.label(RichText::new(&p.text).strong());
        }
        PmiKind::Roughness { ra, process, extra } => crate::pmi_alvos_ui::rugosidade_ui(ui, ra, process, extra),
        PmiKind::DatumTarget { alvo } => {
            crate::pmi_alvos_ui::alvo_ui(ui, alvo, &todas, id);
        }
        PmiKind::Weld { kind, size, all_around, extra } => {
            weld_ui(ui, "pmi", kind, size, all_around, extra);
            crate::pmi_alvos_ui::solda_sistema_ui(ui, extra);
        }
        PmiKind::Datum { letter } => {
            ui.horizontal(|ui| {
                ui.label(tr("Letra"));
                crate::pmi_gdt_ui::campo_letra(ui, letter, &outras);
            });
            ui.separator();
            crate::pmi_gdt_ui::painel_consultor(ui, &todas);
        }
        PmiKind::TitleBlock { template } => {
            ui.label(tr("Modelo (os campos entre chaves são preenchidos com os dados da peça)"));
            ui.add(egui::TextEdit::multiline(template).desired_rows(4).desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                for f in ["{nome}", "{codigo}", "{descricao}", "{material}", "{massa}", "{volume}", "{data}", "{autor}"] {
                    if ui.small_button(f).on_hover_text(tr("inserir o campo")).clicked() {
                        template.push_str(f);
                    }
                }
            });
            if ui.small_button(tr("↺ Modelo padrão")).clicked() {
                *template = TITLE_TEMPLATE.into();
            }
            ui.label(RichText::new(tr("Propriedades personalizadas da peça também valem, pelo nome em minúsculas: {revisao}, {cliente}…")).weak().small());
        }
        PmiKind::Balloon { item, manual } => {
            ui.horizontal(|ui| {
                ui.label(tr("Item"));
                ui.add(egui::TextEdit::singleline(item).desired_width(60.));
            });
            ui.checkbox(manual, tr("Feito à mão (os balões automáticos preservam)")).on_hover_text(tr("Marcado: o comando Balões automáticos não troca este balão e não cria outro com o mesmo número."));
        }
        PmiKind::Gdt { ch, tol, datums, extra } => {
            gdt_ui(ui, "pmi_gdt", ch, tol, datums, extra);
            ui.separator();
            crate::pmi_gdt_ui::painel_consultor(ui, &todas);
        }
        _ => {
            ui.horizontal(|ui| {
                ui.label(tr("Texto (vazio = medido)"));
                ui.text_edit_singleline(&mut p.text);
            });
            ui.horizontal(|ui| {
                ui.label(tr("Tolerância +"));
                ui.add(egui::TextEdit::singleline(&mut p.tol_plus).desired_width(60.));
                ui.label("−");
                ui.add(egui::TextEdit::singleline(&mut p.tol_minus).desired_width(60.));
            });
        }
    }
    // setas da nota: alvos, ancoragem e ponta de cada uma (B368)
    let ped_setas = crate::pmi_setas::tem_setas(&p.kind).then(|| crate::pmi_setas::secao(ui, p, &alvos_txt, n_escolhas));
    crate::editor::section(ui, "Aparência");
    crate::pmi_draw::style_ui(ui, &mut p.style, true);
    if ui.small_button(tr("🎨 Estilo geral das anotações…")).clicked() {
        *style_req = true;
    }
    if !matches!(p.kind, PmiKind::TitleBlock { .. } | PmiKind::Weld { .. } | PmiKind::Balloon { .. }) {
        ui.horizontal(|ui| {
            ui.label(tr("Prefixo"));
            ui.add(egui::TextEdit::singleline(&mut p.style.prefix).desired_width(60.).hint_text("4×"));
            ui.label(tr("Sufixo"));
            ui.add(egui::TextEdit::singleline(&mut p.style.suffix).desired_width(60.).hint_text(tr("típ.")));
        });
    }
    ui.horizontal(|ui| {
        ui.label(tr("Pasta"));
        ui.add(egui::TextEdit::singleline(&mut p.folder).desired_width(140.).hint_text(tr("(raiz)")));
    });
    crate::editor::section(ui, "Posição do texto");
    ui.horizontal(|ui| {
        for (k, l) in ["X", "Y", "Z"].iter().enumerate() {
            ui.label(*l);
            { let __r = ui.add(crate::ui_ext::dv(&mut p.offset[k]).speed(0.5)); crate::ui_ext::wheel(ui, &__r, &mut p.offset[k]); __r };
        }
    });
    crate::editor::section(ui, "Orientação do texto");
    let mut on_plane = p.plane.is_some();
    ui.horizontal(|ui| {
        if ui.radio(!on_plane, tr("de frente para a tela")).clicked() {
            on_plane = false;
        }
        if ui.radio(on_plane, tr("deitado num plano")).clicked() {
            on_plane = true;
        }
    });
    if !on_plane {
        p.plane = None;
    } else {
        if p.plane.is_none() {
            p.plane = Some(([0., 0., 1.], [1., 0., 0.]));
        }
        ui.horizontal(|ui| {
            for (n, pl) in [("XY", ([0., 0., 1.], [1., 0., 0.])), ("XZ", ([0., -1., 0.], [1., 0., 0.])), ("YZ", ([1., 0., 0.], [0., 1., 0.]))] {
                if ui.selectable_label(p.plane == Some(pl), n).clicked() {
                    p.plane = Some(pl);
                }
            }
            if ui.add_enabled(seed_plane.is_some(), egui::Button::new(tr("⊙ Plano/face selecionado"))).on_disabled_hover_text(tr("clique numa face ou plano na viewport")).clicked() {
                p.plane = seed_plane;
            }
        });
    }
    ui.label(RichText::new(tr("Arraste o rótulo na viewport para mudar a posição.")).weak().small());
    crate::editor::check(ui, &mut p.visible, "visível");
    if seed_pts.len() >= p.kind.anchors() && ui.button(tr("⊙ Reancorar na seleção atual")).clicked() {
        match reseed {
            Some((a, t)) => {
                p.anchors = a;
                p.targets = t;
            }
            None => {
                p.anchors = seed_pts[..p.kind.anchors()].to_vec();
                p.targets = vec![];
            }
        }
    }
    if let Some(ped) = ped_setas {
        if let Some(k) = ped.remover {
            if let Some(pm) = list_mut(app).iter_mut().find(|x| x.id == id) {
                fenkra_core::pmi_leader::remove_tip(pm, k);
            }
        }
        if let Some(k) = ped.apontar {
            aplicar_pontas(app, Some((id, k)));
        }
        if ped.adicionar {
            *PONTA_PARA.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
            set_variant(None);
            app.picks.clear();
            app.pmi_pending = Some(16);
            app.toast(tr("Adicionar setas: clique faces ou arestas; Ctrl ou Shift somam várias e Enter conclui").to_string(), false);
        }
        if app.in_asm() {
            app.asm.as_mut().unwrap().modified = true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo(nome: &str) -> RecomputeResult {
        let e = crate::examples::all().into_iter().find(|e| e.name == nome).unwrap();
        fenkra_solidos::recompute(&e.doc)
    }

    fn alvos(r: &RecomputeResult) -> Vec<PmiTarget> {
        r.bodies.iter().flat_map(|b| (0..b.mesh.face_info.len()).filter_map(|f| face_target(&b.mesh, f))).collect()
    }

    fn pmi(kind: PmiKind, targets: Vec<Option<PmiTarget>>) -> Pmi {
        Pmi { id: Id(1), name: "t".into(), kind, anchors: vec![], offset: [0.; 3], text: String::new(), tol_plus: String::new(), tol_minus: String::new(), visible: true, plane: None, style: Default::default(), folder: String::new(), targets }
    }

    #[test]
    fn cursor_reconhece_furo_e_cilindro() {
        let r = exemplo("Flange");
        let t = alvos(&r);
        let nomes: Vec<String> = t.iter().map(PmiTarget::label).collect();
        assert!(nomes.iter().any(|n| n == "furo Ø8"), "{nomes:?}");
        assert!(nomes.iter().any(|n| n == "furo Ø32"), "{nomes:?}");
        assert!(nomes.iter().any(|n| n == "cilindro Ø90"), "{nomes:?}");
        // a cota de diâmetro ligada ao furo mede pela superfície, não por um ponto clicado
        let furo = *t.iter().find(|t| t.label() == "furo Ø8").unwrap();
        let mut pm = pmi(PmiKind::Diameter, vec![Some(furo), None]);
        let d = pm.diagnose(&t);
        assert!(d.ok(), "{d:?}");
        let v = pm.measure_targets(&d.resolved).unwrap();
        assert!((v - 8.).abs() < 0.05, "{v}");
        assert_eq!(pm.display(Some(v)), "Ø8");
        pm.targets[0] = None;
        assert!(pm.measure_targets(&pm.diagnose(&t).resolved).is_none());
    }

    #[test]
    fn cota_entre_faces_da_placa_mede_a_espessura() {
        let r = exemplo("Placa simples");
        let t = alvos(&r);
        let topo = *t.iter().find(|t| t.class == TargetClass::FacePlana && t.dir[2] > 0.99).unwrap();
        let base = *t.iter().find(|t| t.class == TargetClass::FacePlana && t.dir[2] < -0.99).unwrap();
        let pm = pmi(PmiKind::Linear, vec![Some(base), Some(topo)]);
        let d = pm.diagnose(&t);
        assert!(d.ok(), "{d:?}");
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for b in &r.bodies {
            for p in b.mesh.positions.chunks_exact(3) {
                lo = lo.min(p[2] as f64);
                hi = hi.max(p[2] as f64);
            }
        }
        let v = pm.measure_targets(&d.resolved).unwrap();
        assert!((v - (hi - lo)).abs() < 1e-3, "{v} x {}", hi - lo);
        // as âncoras da cota ficam alinhadas: a linha mede exatamente a distância entre os planos
        let mut a = vec![[3., 4., lo], [30., 40., hi]];
        fix_pair(&PmiKind::Linear, &mut a, &d.resolved);
        assert_eq!((a[1][0], a[1][1]), (3., 4.));
    }

    #[test]
    fn plano_de_anotacao_e_cota_de_lado() {
        // a cota que corre ao longo da normal do plano não aparece nele
        assert!(plane_hides([0., 0., 1.], &[[0., 0., 0.], [0., 0., 20.]]));
        assert!(!plane_hides([0., 0., 1.], &[[0., 0., 0.], [20., 0., 0.]]));
        assert_eq!(plane_name(Some(([0., 0., 1.], [1., 0., 0.]))), "Plano XY");
        // texto alinhado à geometria: x segue a linha entre as âncoras projetada no plano
        let x = align_x([0., 0., 1.], &[[0., 0., 0.], [3., 4., 9.]]).unwrap();
        assert!((x[0] - 0.6).abs() < 1e-6 && (x[1] - 0.8).abs() < 1e-6 && x[2].abs() < 1e-6);
        // planos candidatos: a face base, as vizinhas e os três principais
        let r = exemplo("Placa simples");
        let (body, face) = r.bodies.iter().enumerate().find_map(|(b, bd)| bd.mesh.face_info.iter().position(|f| f.kind == 0).map(|f| (b, f))).unwrap();
        let c = plane_cands(&r, Some((body, face)), 0, [0.; 3]);
        assert!(c.len() >= 4 && c[0].name == "face base", "{}", c.len());
        assert_eq!(c.iter().filter(|p| p.name.starts_with("Plano")).count(), 3);
        // trocar a face base muda o primeiro candidato
        let c2 = plane_cands(&r, Some((body, face)), 1, [0.; 3]);
        assert_ne!(c[0].n, c2[0].n);
    }
}

#[cfg(test)]
mod tests_b185_b186 {
    use super::*;
    use fenkra_core::doc::FeatureKind;

    #[test]
    fn tab_percorre_as_variantes_e_o_comando_novo_nasce_do_indice() {
        // índice fora da barra para não pisar nos outros testes (o estado é da sessão)
        set_variant(Some((200, PmiKind::Linear.next_variant())));
        assert_eq!(kind_now(200), PmiKind::Ordinate { axis: 0 });
        assert_eq!(kind_now(0), PmiKind::Linear, "outro comando não herda a variante");
        set_variant(None);
        assert_eq!(kind_now(200), kind_of(200));
        assert_eq!(kind_of(11), PmiKind::Ordinate { axis: 0 });
        assert_eq!(kind_of(12), PmiKind::ArcLength);
        assert!(is_multi(13) && is_multi(14) && !is_multi(0));
        assert!(matches!(kind_of(15), PmiKind::FeatureNote { .. }));
    }

    #[test]
    fn contagem_da_chamada_dos_furos_confere_com_a_geometria() {
        let e = crate::examples::all().into_iter().find(|e| e.name == "Placa paramétrica").unwrap();
        let r = fenkra_solidos::recompute(&e.doc);
        let mut conferidos = 0;
        for (_, f) in e.doc.active_features() {
            if !matches!(f.kind, FeatureKind::Hole(_)) {
                continue;
            }
            let c = fenkra_core::pmi_feature::callout(&e.doc, f.id, true).expect("o furo tem chamada");
            let (Some(d), n) = (c.hole_diameter, c.count.unwrap_or(1)) else { continue };
            let achados = holes_found(&r, f.id, d);
            assert_eq!(achados, n, "{}: chamada «{}» declara {n}× e a geometria tem {achados}", f.name, c.text);
            assert_eq!(fenkra_core::pmi_feature::count_state(n, achados).0, PmiState::Ok);
            // um furo a menos na geometria deixa a chamada ambígua
            assert_eq!(fenkra_core::pmi_feature::count_state(n + 1, achados).0, PmiState::Ambigua);
            conferidos += 1;
        }
        assert!(conferidos > 0, "a placa paramétrica tem furos");
    }
}
