// SPDX-License-Identifier: LGPL-2.1-or-later
//! Converte o resultado do recompute em uma cena para a viewport.
//!
//! Visibilidade: cada item tem oculto/visível; o **ambiente de ocultos** inverte a regra (mostra só o
//! que está oculto, para reexibir). O **filtro** de exibição vale nos dois ambientes.

use crate::app::Sel;
use crate::render::{LineVertex, MeshVertex, Scene};
use crate::settings::Filter;
use eframe::egui;
use glam::Vec3;
use fenkra_core::{Appearance, Document, Id, OriginPlane, PlaneRef, RefKind};
use fenkra_solidos::RecomputeResult;
use std::collections::{HashMap, HashSet};

const SKETCH: [f32; 4] = [0.35, 0.75, 1.0, 1.0];
const CONSTR: [f32; 4] = [0.85, 0.6, 0.3, 0.8];
const HILITE: [f32; 4] = [1.0, 0.85, 0.2, 1.0];

pub struct SceneOpts<'a> {
    /// sketch em edição: suas linhas são desenhadas pelo editor, não pela cena
    pub editing: Option<Id>,
    pub sel: &'a Sel,
    pub filter: &'a Filter,
    pub hidden_env: bool,
    pub dark_edges: bool,
    /// mostrar as arestas de tangência (fim dos raios)
    pub tangent_edges: bool,
    /// item realçado: o que está sob o mouse na árvore, ou o comando aberto
    pub hilite: Option<Id>,
    /// comandos realçados além de `hilite` (linha da lista de corte)
    pub hilite_extra: &'a [Id],
    /// estilo padrão das curvas de referência (px e tipo de linha)
    pub line_w: f32,
    pub line_dash: u8,
    /// símbolo dos pontos: 0 = cruz 3D na cena; outros são desenhados na tela pela viewport
    pub point_style: u8,
    /// forma livre cuja superfície não aparece (modo Gaiola)
    pub forma_oculta: Option<Id>,
    /// mapa de curvatura ou inclinação: o vértice leva a medida do kernel e o shader pinta por pixel
    pub curvatura: Option<crate::sup_inclinacao::EscalaMapa>,
    /// prévia do comando aberto
    pub fantasma: Option<crate::previa::Fantasma<'a>>,
    /// cópias da montagem: esses corpos vão para os lotes, sem repetir a malha na cena (B125)
    pub instancias: &'a [fenkra_montagem::lod::Instancia],
}

/// Linhas da matriz de posição: as colunas são os eixos da peça vistos do mundo.
fn linhas_da_posicao(p: &fenkra_montagem::Placement) -> [[f32; 4]; 3] {
    let e = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].map(|x| p.dir(x));
    [0, 1, 2].map(|r| [e[0][r] as f32, e[1][r] as f32, e[2][r] as f32, p.origin[r] as f32])
}

/// Cópia da cena para um corpo da montagem.
pub(crate) fn copia_de(i: &fenkra_montagem::lod::Instancia, col: [f32; 3], mat: [f32; 4], aresta: [f32; 4]) -> (crate::instancias::Copia, (Vec3, Vec3)) {
    let (lo, hi) = fenkra_montagem::lod::caixa_posta(&i.prototipo.caixa, &i.posicao);
    let (lo, hi) = (Vec3::from(lo), Vec3::from(hi));
    let gpu = crate::instancias::InstanciaGpu { m: linhas_da_posicao(&i.posicao), col: [col[0], col[1], col[2], 1.], mat, aresta };
    (crate::instancias::Copia { gpu, centro: (lo + hi) / 2., raio: (hi - lo).length() / 2., simplificada: i.simplificada }, (lo, hi))
}

fn lv(p: [f32; 3], col: [f32; 4]) -> LineVertex {
    LineVertex { pos: p, col }
}

fn grow(b: &mut Option<(Vec3, Vec3)>, p: Vec3) {
    *b = Some(match *b {
        None => (p, p),
        Some((lo, hi)) => (lo.min(p), hi.max(p)),
    });
}

/// Cores dos eixos X, Y e Z (Origem e referencial dos pontos).
const EIXO_COR: [[f32; 4]; 3] = [[1., 0.3, 0.3, 1.], [0.3, 0.9, 0.3, 1.], [0.35, 0.55, 1., 1.]];

/// Eixos (X, Y, Z) do referencial em que o ponto foi definido: o plano de `frame` do ponto
/// por coordenadas, ou o referencial global nos demais.
pub fn eixos_do_ponto(doc: &Document, r: &RecomputeResult, id: Id) -> [Vec3; 3] {
    let global = [Vec3::X, Vec3::Y, Vec3::Z];
    let Some(RefKind::PointCoords { frame: Some(pl), .. }) = doc.reference(id).map(|x| &x.kind) else { return global };
    let fr = match pl {
        PlaneRef::Origin(o) => Some(fenkra_solidos::origin_frame(*o)),
        PlaneRef::Datum(d) => r.planes.iter().find(|p| p.id == Some(*d)).map(|p| p.frame.clone()),
    };
    match fr {
        Some(f) => [Vec3::from(f3(f.x)), Vec3::from(f3(f.y())), Vec3::from(f3(f.normal))],
        None => global,
    }
}

/// Três setas (linha e cabeça de duas barbas) saindo de `c`, uma por eixo do referencial.
pub fn setas_do_referencial(dst: &mut Vec<LineVertex>, c: Vec3, eixos: [Vec3; 3], comp: f32) {
    for (k, ax) in eixos.iter().enumerate() {
        let col = EIXO_COR[k];
        let ponta = c + *ax * comp;
        let lado = eixos[(k + 1) % 3];
        let cabeca = comp * 0.22;
        dst.push(lv(c.into(), col));
        dst.push(lv(ponta.into(), col));
        for sinal in [1., -1.] {
            dst.push(lv(ponta.into(), col));
            dst.push(lv((ponta - *ax * cabeca + lado * (cabeca * 0.45 * sinal)).into(), col));
        }
    }
}

/// Regra de exibição: no ambiente normal mostra os visíveis; no de ocultos, só os ocultos.
fn shows(hidden: bool, env: bool) -> bool {
    hidden == env
}

fn f3(v: [f64; 3]) -> [f32; 3] {
    v.map(|x| x as f32)
}

fn rgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| x as f32 / 255.)
}

pub fn build(doc: &Document, r: &RecomputeResult, o: &SceneOpts) -> Scene {
    let mut s = Scene::default();
    let mut bounds = None;
    let env = o.hidden_env;

    // realce dinâmico: só o item sob o mouse (ou o comando em edição); a seleção sozinha não brilha
    let _ = o.sel;
    let mut hi_features: HashSet<Id> = HashSet::new();
    if let Some(id) = o.hilite {
        if let Some(f) = doc.feature(id) {
            hi_features.insert(id);
            hi_features.extend(Document::refs_of(&f.kind));
        }
    }
    hi_features.extend(o.hilite_extra.iter().copied());
    // o conjunto único (B95) acende no 3D o que a árvore tem selecionado, além do item sob o mouse
    let hi = |id: Id| o.hilite == Some(id) || o.hilite_extra.contains(&id);

    let feat_app: HashMap<Id, Appearance> = doc.features.iter().filter_map(|f| f.appearance.map(|a| (f.id, a))).collect();
    // só sketches se ocultam um a um; sólidos aparecem pelo corpo (a última feature junta tudo)
    let feat_hidden: HashSet<Id> = doc.features.iter().filter(|f| doc.sketch_hidden(f) && matches!(f.kind, fenkra_core::FeatureKind::Sketch(_))).map(|f| f.id).collect();
    let (peca_tema, aresta_tema) = crate::theme::piece_colors();
    // o mapa de curvatura colore por vértice: só a malha posicionada de cada corpo sabe disso
    let copias: HashMap<Id, &fenkra_montagem::lod::Instancia> = if o.curvatura.is_none() && o.fantasma.is_none() { o.instancias.iter().filter_map(|i| r.bodies.get(i.corpo).map(|b| (b.collector, i))).collect() } else { HashMap::new() };
    s.tangentes = o.tangent_edges;
    let edge_col = if o.dark_edges { aresta_tema.map(|a| { let f = rgb(a); [f[0], f[1], f[2], 1.0] }).unwrap_or([0.06, 0.07, 0.09, 1.0]) } else { [0.85, 0.88, 0.92, 1.0] };

    if o.filter.corpos {
        // o comando refez o corpo inteiro (dobra, joggle…): por baixo do fantasma fica a peça de
        // antes, senão a pessoa perde a referência do que vai mudar
        let fid = o.fantasma.as_ref().map(|f| f.id);
        let camadas = r.bodies.iter().flat_map(|b| {
            let antes = o.fantasma.as_ref().and_then(|f| f.antes_de(b)).map(|c| (c.collector, &c.mesh, c.face_owner.as_slice()));
            // a prévia pinta sobre a malha de trabalho, e a de tela passaria por cima dela
            let tela = r.exibicao.get(&b.collector).filter(|_| o.fantasma.is_none());
            antes.into_iter().chain(std::iter::once((b.collector, tela.map_or(&b.mesh, |m| &**m), b.face_owner.as_slice())))
        });
        for (collector, m, face_owner) in camadas {
            let Some(col) = doc.collectors.iter().find(|c| c.id == collector) else { continue };
            let body_sel = hi(collector);
            let base = doc.appearance.unwrap_or(Appearance { color: col.color, finish: col.finish, texture: col.texture });
            if let Some(i) = copias.get(&collector) {
                if shows(!col.visible, env) {
                    let (rough, metal, alpha) = base.finish.params();
                    let mut c = rgb(if base.color == crate::theme::PECA_PADRAO { peca_tema.unwrap_or(base.color) } else { base.color });
                    if body_sel {
                        c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
                    }
                    let textura = base.texture.filter(|t| (*t as u32) < crate::render::TEX_LAYERS).map(|t| t as f32 + 1.).unwrap_or(0.);
                    let (copia, (lo, hi)) = copia_de(i, c, [rough, metal, alpha, textura], edge_col);
                    grow(&mut bounds, lo);
                    grow(&mut bounds, hi);
                    match s.lotes.iter_mut().find(|l| l.prototipo.id == i.prototipo.id) {
                        Some(l) => l.copias.push(copia),
                        None => s.lotes.push(crate::instancias::Lote { prototipo: i.prototipo.clone(), copias: vec![copia] }),
                    }
                }
                continue;
            }
            let nfaces = m.face_ranges.len();
            for fi in 0..nfaces {
                let owner = face_owner.get(fi).copied();
                let face_hidden = !col.visible || owner.is_some_and(|w| feat_hidden.contains(&w));
                if !shows(face_hidden, env) {
                    continue;
                }
                if let (Some(f), true) = (&o.fantasma, owner.is_some() && owner == fid) {
                    crate::previa::face_fantasma(&mut s.ghost, m, fi, f.cor_de(doc));
                    crate::previa::crescer_com_face(m, fi, &mut |p| grow(&mut bounds, p));
                    continue;
                }
                let app = owner.and_then(|w| feat_app.get(&w).copied()).unwrap_or(base);
                let (rough, metal, alpha) = app.finish.params();
                // corpo na cor padrão do documento ganha a cor de peça do tema
                let mut c = rgb(if app.color == crate::theme::PECA_PADRAO { peca_tema.unwrap_or(app.color) } else { app.color });
                if body_sel || owner.is_some_and(|w| hi_features.contains(&w)) {
                    c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
                }
                let start = m.face_ranges[fi] as usize;
                let end = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
                let textura = app.texture.filter(|t| (*t as u32) < crate::render::TEX_LAYERS).map(|t| t as f32 + 1.).unwrap_or(0.);
                let medida = |i: usize| o.curvatura.and_then(|e| e.atributo_da_malha(m, i));
                // duplica os vértices da face para poder colorir por face
                let mut remap: HashMap<u32, u32> = HashMap::new();
                for &vi in &m.indices[start..end] {
                    let ni = *remap.entry(vi).or_insert_with(|| {
                        let k = vi as usize * 3;
                        let p = [m.positions[k], m.positions[k + 1], m.positions[k + 2]];
                        grow(&mut bounds, Vec3::from(p));
                        s.mesh.push(MeshVertex {
                            pos: p,
                            nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]],
                            col: {
                                let c = medida(vi as usize).unwrap_or(c);
                                [c[0], c[1], c[2], 1.0]
                            },
                            mat: [rough, metal, alpha, if medida(vi as usize).is_some() { crate::render::MAPA_POR_PIXEL } else { textura }],
                        });
                        (s.mesh.len() - 1) as u32
                    });
                    s.indices.push(ni);
                }
            }
            if shows(!col.visible, env) {
                let cor = o.fantasma.as_ref().map(|f| f.cor_de(doc));
                match (fid, cor) {
                    (Some(id), Some(c)) if face_owner.contains(&id) => crate::previa::arestas_com_fantasma(&mut s.edges, &mut s.lines, m, o.tangent_edges, edge_col, c, face_owner, id),
                    _ => push_edges(&mut s.edges, m, o.tangent_edges, edge_col),
                }
            }
        }
    }

    // sketches: mostrados conforme oculto/visível; o sketch do comando selecionado sempre aparece em destaque
    if o.filter.sketches {
        for sk in &r.sketches {
            let hidden = feat_hidden.contains(&sk.feature);
            let editing = o.editing == Some(sk.feature);
            let h = hi_features.contains(&sk.feature) && !editing;
            if !editing && !h && !shows(hidden, env) {
                continue;
            }
            // sombra dos perfis fechados
            let n = Vec3::from(sk.frame.normal.map(|v| v as f32));
            let fc = if editing { [0.3, 0.65, 1.0, 1.0] } else { [0.35, 0.7, 1.0, 1.0] };
            for p in &sk.fill {
                s.fill.push(MeshVertex { pos: *p, nrm: n.into(), col: fc, mat: [1.0, 0.0, if editing { 0.22 } else { 0.14 }, 0.] });
            }
            if editing {
                continue;
            }
            // curvas com estilo próprio (espessura/tipo de linha) vão pelo traço grosso
            let styled: Vec<&(Id, bool, Vec<[f32; 3]>)> = sk.ents.iter().filter(|(cid, ..)| doc.line_style(*cid).is_some()).collect();
            if styled.is_empty() {
                let dst = if h { &mut s.overlay } else { &mut s.lines };
                for p in &sk.lines {
                    dst.push(lv(*p, if h { HILITE } else { SKETCH }));
                    grow(&mut bounds, Vec3::from(*p));
                }
                for p in &sk.construction {
                    dst.push(lv(*p, CONSTR));
                }
            } else {
                for (cid, cons, pts) in &sk.ents {
                    for p in pts {
                        grow(&mut bounds, Vec3::from(*p));
                    }
                    let col = if h { HILITE } else if *cons { CONSTR } else { SKETCH };
                    match doc.line_style(*cid) {
                        Some(st) => crate::render::push_wide(&mut s.wide, pts, col, st.width, st.dash),
                        None => {
                            let dst = if h { &mut s.overlay } else { &mut s.lines };
                            for w in pts.windows(2) {
                                dst.push(lv(w[0], col));
                                dst.push(lv(w[1], col));
                            }
                        }
                    }
                }
            }
        }
    }

    // tamanho de referência para planos, pontos e eixos
    let size = bounds.map(|(lo, hi): (Vec3, Vec3)| (hi - lo).max_element().max(20.) * 0.6).unwrap_or(60.);

    if o.filter.sketches && o.filter.pontos {
        for sk in &r.sketches {
            let h = hi_features.contains(&sk.feature);
            if !h && !shows(feat_hidden.contains(&sk.feature), env) {
                continue;
            }
            let d = size * 0.025;
            if o.point_style != 0 {
                continue;
            }
            for p in &sk.points {
                let c = Vec3::from(*p);
                for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                    s.overlay.push(lv((c - ax * d).into(), if h { HILITE } else { SKETCH }));
                    s.overlay.push(lv((c + ax * d).into(), if h { HILITE } else { SKETCH }));
                }
            }
        }
    }

    if let Some(f) = &o.fantasma {
        crate::previa::decorar(doc, r, f, &mut s);
    }

    // planos da Origem e das Referências
    for p in &r.planes {
        let (visible_flag, allowed) = match p.id {
            Some(id) => (doc.reference(id).is_some_and(|x| doc.ref_shown(x)), o.filter.planos_ref),
            None => (doc.origin_visible, o.filter.planos_origem),
        };
        let selected = match (o.sel, p.id) {
            (_, Some(b)) => hi(b),
            (Sel::Origin(op), None) => p.name == plane_name(*op),
            _ => false,
        };
        if !selected && (!allowed || !shows(!visible_flag, env)) {
            continue;
        }
        let col = match (p.id, p.name.as_str()) {
            (Some(_), _) => [0.95, 0.7, 0.3, 0.6],
            (_, "XY") => [0.4, 0.55, 1.0, 0.5],
            (_, "XZ") => [0.4, 0.85, 0.45, 0.5],
            _ => [1.0, 0.45, 0.4, 0.5],
        };
        let f = &p.frame;
        let (o3, x, yv) = (
            Vec3::from(f.origin.map(|v| v as f32)),
            Vec3::from(f.x.map(|v| v as f32)),
            Vec3::from(f.y().map(|v| v as f32)),
        );
        let h = size * if p.id.is_some() { 0.45 } else { 0.5 };
        let c = [o3 + (x + yv) * h, o3 + (x - yv) * h, o3 + (-x - yv) * h, o3 + (-x + yv) * h];
        // B228: o plano do comando aberto é um retângulo translúcido com borda, não só um contorno
        let previa = o.fantasma.as_ref().filter(|f| Some(f.id) == p.id);
        if let Some(f) = previa {
            crate::previa::retangulo_fantasma(&mut s.ghost, c, Vec3::from(f3(p.frame.normal)), f.cor);
        }
        let dst = if selected || previa.is_some() { &mut s.overlay } else { &mut s.lines };
        for i in 0..4 {
            let colr = previa.map(|f| crate::previa::cor_linha(f.cor)).unwrap_or(if selected { HILITE } else { col });
            dst.push(lv(c[i].into(), colr));
            dst.push(lv(c[(i + 1) % 4].into(), colr));
        }
    }

    // superfícies: objetos opacos (como os corpos), com cor própria e contorno
    if o.filter.planos_ref {
        for sf in &r.surfaces {
            // o corte por região desenha os pedaços; a superfície inteira ficaria na frente deles
            if o.fantasma.as_ref().is_some_and(|f| crate::sup3d::entrada_das_regioes(doc, f.id, sf.id)) {
                continue;
            }
            let selected = hi(sf.id);
            let rf = doc.reference(sf.id);
            let vis = rf.is_some_and(|x| doc.ref_shown(x));
            if !selected && !shows(!vis, env) {
                continue;
            }
            if let Some(f) = o.fantasma.as_ref().filter(|f| f.id == sf.id) {
                for fi in 0..sf.mesh.face_ranges.len() {
                    if crate::sup3d::regioes(r, f.id).is_empty() {
                        crate::previa::face_fantasma(&mut s.ghost, &sf.mesh, fi, f.cor);
                    }
                    crate::previa::crescer_com_face(&sf.mesh, fi, &mut |p| grow(&mut bounds, p));
                }
                continue;
            }
            let app = rf.and_then(|x| x.appearance).unwrap_or(Appearance { color: [222, 170, 110], finish: fenkra_core::Finish::Acetinado, texture: None });
            let (rough, metal, alpha) = app.finish.params();
            let mut c = rgb(app.color);
            if selected {
                c = [c[0] * 0.55 + 0.45, c[1] * 0.55 + 0.38, c[2] * 0.55 + 0.08];
            }
            let m = r.exibicao.get(&sf.id).filter(|_| o.fantasma.is_none()).map_or(&sf.mesh, |m| &**m);
            let base = s.mesh.len() as u32;
            for k in (0..m.positions.len()).step_by(3) {
                let p = [m.positions[k], m.positions[k + 1], m.positions[k + 2]];
                grow(&mut bounds, Vec3::from(p));
                let medida = o.curvatura.and_then(|e| e.atributo_da_malha(m, k / 3));
                let c = medida.unwrap_or(c);
                s.mesh.push(MeshVertex { pos: p, nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]], col: [c[0], c[1], c[2], 1.0], mat: [rough, metal, alpha, if medida.is_some() { crate::render::MAPA_POR_PIXEL } else { 0. }] });
            }
            s.indices.extend(m.indices.iter().map(|i| base + i));
            push_edges(&mut s.edges, m, o.tangent_edges, if selected { HILITE } else { edge_col });
        }
    }

    crate::forma_ui::na_cena(doc, r, o.forma_oculta, o.curvatura.as_ref(), &mut s, &mut |p| grow(&mut bounds, p));

    // pontos, linhas e curvas de referência
    let geo_col = [0.95, 0.7, 0.3, 1.0];
    for c in &r.curves {
        let selected = hi(c.id);
        let vis = doc.reference(c.id).is_some_and(|x| doc.ref_shown(x));
        if !selected && (!o.filter.planos_ref || !shows(!vis, env)) {
            continue;
        }
        // espessura e tipo de linha: o do elemento, senão o padrão das configurações
        let st = doc.line_style(c.id);
        let (w, dash) = st.map(|x| (x.width, x.dash)).unwrap_or((o.line_w, o.line_dash));
        let col = match (selected, doc.reference(c.id).and_then(|x| x.appearance)) {
            (true, _) => HILITE,
            (false, Some(a)) => {
                let k = rgb(a.color);
                [k[0], k[1], k[2], 1.0]
            }
            _ => geo_col,
        };
        let pts: Vec<[f32; 3]> = c.pts.iter().map(|p| p.map(|v| v as f32)).collect();
        crate::render::push_wide(&mut s.wide, &pts, col, if selected { w + 1.5 } else { w }, dash);
        if selected {
            for w2 in c.pts.windows(2) {
                s.overlay.push(lv(w2[0].map(|v| v as f32), HILITE));
                s.overlay.push(lv(w2[1].map(|v| v as f32), HILITE));
            }
        }
    }
    if o.filter.pontos && o.point_style == 0 {
        for p in &r.points {
            let selected = hi(p.id);
            let vis = doc.reference(p.id).is_some_and(|x| doc.ref_shown(x));
            if !selected && !shows(!vis, env) {
                continue;
            }
            let c = Vec3::from(p.p.map(|v| v as f32));
            let d = size * 0.03;
            for ax in [Vec3::X, Vec3::Y, Vec3::Z] {
                let col = if selected { HILITE } else { geo_col };
                s.overlay.push(lv((c - ax * d).into(), col));
                s.overlay.push(lv((c + ax * d).into(), col));
            }
        }
    }

    // B24: setas 3D do referencial nos pontos em foco (selecionados ou sob o ponteiro)
    if o.filter.pontos {
        for p in &r.points {
            let em_foco = hi(p.id) || matches!(o.sel, Sel::Ref(x) if *x == p.id);
            let vis = doc.reference(p.id).is_some_and(|x| doc.ref_shown(x));
            if em_foco && (vis || env) {
                setas_do_referencial(&mut s.overlay, Vec3::from(p.p.map(|v| v as f32)), eixos_do_ponto(doc, r, p.id), size * 0.09);
            }
        }
    }

    // eixos da Origem
    if o.filter.eixos && shows(!doc.origin_visible, env) {
        for (ax, col) in [(Vec3::X, EIXO_COR[0]), (Vec3::Y, EIXO_COR[1]), (Vec3::Z, EIXO_COR[2])] {
            s.overlay.push(lv([0.; 3], col));
            s.overlay.push(lv((ax * size * 0.25).into(), col));
        }
    }

    s.bounds = bounds.or(Some((Vec3::splat(-size), Vec3::splat(size))));
    s
}

/// O comando é um sketch?
pub fn is_sketch_feature(doc: &Document, id: Id) -> bool {
    doc.feature(id).is_some_and(|f| matches!(f.kind, fenkra_core::FeatureKind::Sketch(_)))
}

pub fn plane_name(o: OriginPlane) -> &'static str {
    match o {
        OriginPlane::XY => "XY",
        OriginPlane::XZ => "XZ",
        OriginPlane::YZ => "YZ",
    }
}

pub fn plane_ref_name(doc: &Document, p: &PlaneRef) -> String {
    match p {
        PlaneRef::Origin(o) => format!("Origem · {}", plane_name(*o)),
        PlaneRef::Datum(id) => doc.reference(*id).map(|r| r.name.clone()).or_else(|| crate::slots::label(*id)).unwrap_or_else(|| format!("⚠ perdida #{}", id.0)),
    }
}

/// Símbolos dos pontos (referência e centros de sketch) em tamanho fixo na tela, no estilo
/// escolhido (asterisco, X, círculo, quadrado, ponto) — o estilo 0 (cruz 3D) fica na cena.
#[allow(clippy::too_many_arguments)]
pub fn draw_points(painter: &egui::Painter, doc: &Document, r: &RecomputeResult, cam: &crate::render::Camera, rect: egui::Rect, style: u8, size: f32, filter: &Filter, hidden_env: bool, editing: Option<Id>, hilite: Option<Id>, sel: &Sel) {
    if style == 0 || !filter.pontos {
        return;
    }
    let geo = egui::Color32::from_rgb(242, 178, 76);
    let sk_col = egui::Color32::from_rgb(90, 170, 255);
    let hl = egui::Color32::from_rgb(255, 205, 60);
    let draw = |p: [f32; 3], col: egui::Color32, st: u8, sz: f32| {
        let Some(c) = cam.project(rect, Vec3::from(p)) else { return };
        if !rect.contains(c) {
            return;
        }
        let h = sz / 2.;
        let stroke = egui::Stroke::new(1.6, col);
        match st {
            1 => {
                for k in 0..3 {
                    let a = k as f32 * std::f32::consts::PI / 3. + std::f32::consts::FRAC_PI_2;
                    let d = egui::vec2(a.cos(), -a.sin()) * h;
                    painter.line_segment([c - d, c + d], stroke);
                }
            }
            2 => {
                painter.line_segment([c + egui::vec2(-h, -h), c + egui::vec2(h, h)], stroke);
                painter.line_segment([c + egui::vec2(-h, h), c + egui::vec2(h, -h)], stroke);
            }
            3 => {
                painter.circle_stroke(c, h, stroke);
                painter.circle_filled(c, 1.2, col);
            }
            4 => {
                painter.rect_stroke(egui::Rect::from_center_size(c, egui::vec2(sz, sz)), 0., stroke, egui::StrokeKind::Middle);
            }
            _ => {
                painter.circle_filled(c, h * 0.5, col);
            }
        }
    };
    for p in &r.points {
        let rf = doc.reference(p.id);
        let vis = rf.is_some_and(|x| doc.ref_shown(x));
        let selected = hilite == Some(p.id) || matches!(sel, Sel::Ref(x) if *x == p.id);
        if !selected && !shows(!vis, hidden_env) {
            continue;
        }
        let st = doc.line_style(p.id);
        let col = match (selected, rf.and_then(|x| x.appearance)) {
            (true, _) => hl,
            (false, Some(a)) => egui::Color32::from_rgb(a.color[0], a.color[1], a.color[2]),
            _ => geo,
        };
        draw(p.p.map(|v| v as f32), col, st.map(|x| x.point).filter(|x| *x != 0).unwrap_or(style), st.map(|x| x.size).unwrap_or(size));
    }
    if filter.sketches {
        for sk in &r.sketches {
            if Some(sk.feature) == editing {
                continue;
            }
            let hidden = doc.feature(sk.feature).is_some_and(|f| doc.sketch_hidden(f));
            if !shows(hidden, hidden_env) {
                continue;
            }
            let col = if hilite == Some(sk.feature) { hl } else { sk_col };
            for p in &sk.points {
                draw(*p, col, style, size);
            }
        }
    }
}

#[cfg(test)]
mod testes_setas {
    use super::*;

    fn ponto(doc: &mut Document, frame: Option<PlaneRef>) -> Id {
        doc.add_geometry(RefKind::PointCoords { x: "1".into(), y: "2".into(), z: "3".into(), frame })
    }

    #[test]
    fn ponto_sem_referencial_usa_os_eixos_globais() {
        let mut d = Document::new("t");
        let p = ponto(&mut d, None);
        let r = fenkra_solidos::recompute(&d);
        assert_eq!(eixos_do_ponto(&d, &r, p), [Vec3::X, Vec3::Y, Vec3::Z]);
    }

    #[test]
    fn ponto_no_plano_yz_mostra_as_setas_do_referencial_dele() {
        let mut d = Document::new("t");
        let p = ponto(&mut d, Some(PlaneRef::Origin(OriginPlane::YZ)));
        let r = fenkra_solidos::recompute(&d);
        // YZ: x = Y global, normal = X global, y = normal x x = Z global
        let e = eixos_do_ponto(&d, &r, p);
        assert!((e[0] - Vec3::Y).length() < 1e-6 && (e[1] - Vec3::Z).length() < 1e-6 && (e[2] - Vec3::X).length() < 1e-6, "{e:?}");
    }

    #[test]
    fn ponto_num_plano_de_referencia_segue_o_plano() {
        let mut d = Document::new("t");
        let pl = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "5");
        let p = ponto(&mut d, Some(PlaneRef::Datum(pl)));
        let r = fenkra_solidos::recompute(&d);
        let e = eixos_do_ponto(&d, &r, p);
        assert!((e[2] - Vec3::Z).length() < 1e-6, "{e:?}");
    }

    #[test]
    fn setas_tem_tres_eixos_cada_um_com_haste_e_duas_barbas() {
        let mut v = vec![];
        setas_do_referencial(&mut v, Vec3::ZERO, [Vec3::X, Vec3::Y, Vec3::Z], 10.);
        assert_eq!(v.len(), 3 * 6);
        assert_eq!(v[1].pos, [10., 0., 0.]);
    }
}

/// Arestas da malha; as de tangência (suaves) só se pedidas — o raio fica limpo como na peça real.
pub(crate) fn push_edges(dst: &mut Vec<LineVertex>, m: &fenkra_kernel::Mesh, tangent: bool, col: [f32; 4]) {
    let nseg = m.edges.len() / 6;
    if tangent || m.edge_smooth.len() != m.edge_ranges.len() {
        for p in m.edges.chunks_exact(3) {
            dst.push(lv([p[0], p[1], p[2]], col));
        }
        return;
    }
    for (e, &sm) in m.edge_smooth.iter().enumerate() {
        if sm != 0 {
            continue;
        }
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
        for q in m.edges[s0 * 6..s1 * 6].chunks_exact(3) {
            dst.push(lv([q[0], q[1], q[2]], col));
        }
    }
}

#[cfg(test)]
mod testes_malha_de_tela {
    use super::*;
    use crate::app::Sel;
    use crate::render::MAPA_POR_PIXEL;
    use crate::sup_analise::{EscalaCurvatura, TipoCurvatura};
    use crate::sup_inclinacao::EscalaMapa;
    use fenkra_core::{BoolOp, FeatureKind, Polimento, PrimKind, Primitive, QualidadeMalha};

    const PARADAS: [[f32; 3]; 4] = [[0., 0., 1.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]];

    fn esfera(r: &str) -> Document {
        let mut d = Document::new("t");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Esfera, plane: PlaneRef::Origin(OriginPlane::XY), a: r.into(), b: String::new(), c: String::new(), d: String::new(), op: BoolOp::Add }));
        d
    }

    fn polida(d: &Document) -> RecomputeResult {
        let mut c = fenkra_solidos::Cache::com_qualidade(QualidadeMalha { desvio_rel: 0.02, angulo_graus: 45. });
        c.polir.alvo = Some(Polimento { desvio_mm: 0.05, angulo_graus: 5., curvatura: false, teto_triangulos: Polimento::TETO });
        c.polir.agora = true;
        fenkra_solidos::recompute_cached(d, &mut c)
    }

    fn cena(d: &Document, r: &RecomputeResult, curvatura: Option<EscalaMapa>, fantasma: Option<crate::previa::Fantasma>) -> Scene {
        let (sel, filtro) = (Sel::None, Filter::default());
        let o = SceneOpts { instancias: &[], forma_oculta: None, editing: None, sel: &sel, filter: &filtro, hidden_env: false, dark_edges: true, tangent_edges: false, hilite: None, hilite_extra: &[], line_w: 1., line_dash: 0, point_style: 0, curvatura, fantasma };
        build(d, r, &o)
    }

    #[test]
    fn cena_desenha_a_malha_de_tela_e_a_selecao_segue_na_de_trabalho() {
        let d = esfera("40");
        let r = polida(&d);
        let (trabalho, tela) = (r.bodies[0].mesh.indices.len(), r.exibicao[&r.bodies[0].collector].indices.len());
        assert!(tela > trabalho);
        assert_eq!(cena(&d, &r, None, None).indices.len(), tela);
    }

    #[test]
    fn previa_aberta_volta_a_malha_de_trabalho() {
        let d = esfera("40");
        let r = polida(&d);
        let f = crate::previa::Fantasma { id: fenkra_core::Id(u64::MAX), cor: [10, 20, 200], entrada: [200, 150, 10], erro: [200, 0, 0], retirada: [150, 0, 150], ferramenta: [0, 200, 200], neutro: [0, 200, 90], antes: &[] };
        assert_eq!(cena(&d, &r, None, Some(f)).indices.len(), r.bodies[0].mesh.indices.len());
    }

    #[test]
    fn mapa_manda_as_curvaturas_exatas_e_deixa_a_cor_para_o_pixel() {
        let d = esfera("40");
        let r = polida(&d);
        let e = EscalaCurvatura { tipo: TipoCurvatura::Maxima, raio: 40., paradas: PARADAS };
        let s = cena(&d, &r, Some(e.into()), None);
        assert!(!s.mesh.is_empty());
        for v in &s.mesh {
            assert_eq!(v.mat[3], MAPA_POR_PIXEL);
            assert!((v.col[0].abs() - 1. / 40.).abs() < 1e-5 && (v.col[1].abs() - 1. / 40.).abs() < 1e-5, "{:?}", v.col);
        }
        // fora do mapa o vértice leva a cor da peça
        assert!(cena(&d, &r, None, None).mesh.iter().all(|v| v.mat[3] >= 0.));
    }
}
