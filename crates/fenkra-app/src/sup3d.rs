// SPDX-License-Identifier: LGPL-2.1-or-later
//! B276: o que um comando de superfície aberto mostra no 3D além do resultado, que já é a prévia
//! fantasma comum (`previa.rs`). As entradas aparecem na cor do papel delas, as curvas ganham a seta
//! de sentido, as superfícies a normal, a escolha de lado vira seta clicável e a de solução, bolinha.
//!
//! A seta de lado segue a regra do cálculo (`split_side`): fica o que está do lado da normal da
//! ferramenta, ou o oposto com `flip`. Por isso ela sai da ferramenta, no ponto mais perto da
//! superfície cortada, e aponta para o pedaço que fica.

use crate::app::App;
use crate::i18n::tr;
use crate::render::{Camera, Scene};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use fenkra_core::*;
use fenkra_kernel::Mesh;
use fenkra_solidos::RecomputeResult;
use glam::Vec3;

/// O que a entrada é para o comando.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Papel {
    /// superfície onde o comando trabalha
    Base,
    /// elemento que corta
    Ferramenta,
    /// curva que dá a forma
    Curva,
    /// caminho, spine ou guia
    Guia,
    Ponto,
    Plano,
}

impl Papel {
    pub fn rotulo(self) -> &'static str {
        match self {
            Papel::Base => "Superfície",
            Papel::Ferramenta => "Ferramenta",
            Papel::Curva => "Curva",
            Papel::Guia => "Guia",
            Papel::Ponto => "Ponto",
            Papel::Plano => "Plano",
        }
    }

    /// Cor própria do papel no tema; a guia usa o destaque.
    fn propria(self) -> Option<Color32> {
        match self {
            Papel::Base => Some(crate::theme::st_warn()),
            Papel::Ferramenta => Some(crate::theme::papel_ferramenta()),
            Papel::Curva | Papel::Ponto => Some(crate::theme::st_ok()),
            Papel::Guia => None,
            Papel::Plano => Some(crate::theme::papel_neutro()),
        }
    }

    /// Cor do papel, a mesma no painel e no 3D.
    pub fn cor(self, destaque: Color32) -> Color32 {
        self.propria().unwrap_or(destaque)
    }

    pub fn rgb(self, destaque: [u8; 3]) -> [u8; 3] {
        self.propria().map_or(destaque, |c| [c.r(), c.g(), c.b()])
    }
}

/// Elemento de entrada.
#[derive(Clone, PartialEq, Debug)]
pub enum Alvo {
    Sup(SurfRef),
    Curva(CurveRef),
    Ponto(PointRef),
    Plano(PlaneRef),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Suporte {
    pub papel: Papel,
    pub alvo: Alvo,
}

fn sup(papel: Papel, s: &SurfRef) -> Suporte {
    Suporte { papel, alvo: Alvo::Sup(*s) }
}

fn cur(papel: Papel, c: &CurveRef) -> Suporte {
    Suporte { papel, alvo: Alvo::Curva(c.clone()) }
}

/// Entradas do comando, cada uma com o seu papel.
pub fn suportes(doc: &Document, id: Id) -> Vec<Suporte> {
    if let Some(r) = doc.reference(id) {
        return de_referencia(&r.kind);
    }
    match doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::SplitBody(x)) => vec![sup(Papel::Ferramenta, &x.surface)],
        Some(FeatureKind::SewSurface(x)) => vec![sup(Papel::Ferramenta, &x.surface)],
        Some(FeatureKind::ThickSurface(x)) => vec![sup(Papel::Base, &x.surface)],
        Some(FeatureKind::CloseSurface(x)) => vec![sup(Papel::Base, &x.surface)],
        _ => vec![],
    }
}

pub fn de_referencia(k: &RefKind) -> Vec<Suporte> {
    use RefKind as R;
    // o plano em edição tem desenho próprio (B228) e o que ele usa não é escolha de superfície
    if k.class() == RefClass::Plane {
        return vec![];
    }
    match k {
        R::SurfSplit { surface, tool, more, curve, .. } => {
            let mut v = vec![sup(Papel::Base, surface), sup(Papel::Ferramenta, tool)];
            v.extend(more.iter().map(|t| sup(Papel::Ferramenta, t)));
            v.extend(curve.iter().map(|c| cur(Papel::Ferramenta, c)));
            v
        }
        R::SurfRegions { surface, tools, curve, mutual, .. } => {
            let papel = if *mutual { Papel::Base } else { Papel::Ferramenta };
            let mut v = vec![sup(Papel::Base, surface)];
            v.extend(tools.iter().map(|t| sup(papel, t)));
            v.extend(curve.iter().map(|c| cur(Papel::Ferramenta, c)));
            v
        }
        R::SurfTrim { a, b, .. } | R::SurfFillet { a, b, .. } | R::SurfConnect { a, b, .. } | R::CurveIntersect { a, b, .. } => vec![sup(Papel::Base, a), sup(Papel::Base, b)],
        R::SurfJoin { items, curves, .. } => items.iter().map(|s| sup(Papel::Base, s)).chain(curves.iter().map(|c| cur(Papel::Curva, c))).collect(),
        R::SurfEdgeFillet { surface, .. } => vec![sup(Papel::Base, surface)],
        R::SurfSweep { profile, spine, .. } => {
            let mut v = vec![cur(Papel::Guia, spine)];
            if let SweepProfile::Curve(c) = profile {
                v.push(cur(Papel::Curva, c));
            }
            v
        }
        R::SurfSweepX { kind, spine, .. } => kind.guides().into_iter().map(|c| cur(Papel::Curva, c)).chain(spine.iter().map(|c| cur(Papel::Guia, c))).collect(),
        R::SurfLoft { sections, guide, opts, .. } => sections.iter().map(|c| cur(Papel::Curva, c)).chain(guide.iter().chain(&opts.guides).map(|c| cur(Papel::Guia, c))).collect(),
        R::SurfRuled { curves, support, .. } => curves.iter().map(|c| cur(Papel::Curva, c)).chain(support.iter().map(|s| sup(Papel::Base, s))).collect(),
        R::SurfCheck { mode: CheckMode::Pente, curve, .. } => vec![cur(Papel::Curva, curve)],
        R::SurfCheck { a, b, .. } => vec![sup(Papel::Base, a), sup(Papel::Ferramenta, b)],
        _ => {
            let s = crate::geo_ui::inputs_of(k);
            let mut v: Vec<Suporte> = s.surface.iter().map(|x| sup(Papel::Base, x)).collect();
            v.extend(s.curves.iter().map(|c| cur(Papel::Curva, c)));
            v.extend(s.points.iter().map(|p| Suporte { papel: Papel::Ponto, alvo: Alvo::Ponto(*p) }));
            v.extend(s.planes.iter().map(|p| Suporte { papel: Papel::Plano, alvo: Alvo::Plano(*p) }));
            v
        }
    }
}

/// Legenda dos papéis no topo do painel: cada cor uma vez.
pub fn legenda(ui: &mut egui::Ui, itens: &[Suporte]) {
    let destaque = crate::theme::accent(ui.visuals());
    let mut papeis: Vec<Papel> = vec![];
    for s in itens {
        if !papeis.contains(&s.papel) {
            papeis.push(s.papel);
        }
    }
    if papeis.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for p in papeis {
            ui.label(RichText::new("■").color(p.cor(destaque)));
            ui.label(RichText::new(tr(p.rotulo())).small());
        }
    });
}

// ---------------------------------------------------------------- pontos sobre as entradas

fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Ponto da malha mais perto de `p` e a normal ali, com a orientação das normais da malha.
pub fn perto_na_malha(m: &Mesh, p: Vec3) -> Option<(Vec3, Vec3)> {
    let vtx = |a: &[f32], i: u32| Vec3::new(a[i as usize * 3], a[i as usize * 3 + 1], a[i as usize * 3 + 2]);
    let mut melhor: Option<(f32, Vec3, Vec3)> = None;
    for t in m.indices.chunks_exact(3) {
        let (a, b, c) = (vtx(&m.positions, t[0]), vtx(&m.positions, t[1]), vtx(&m.positions, t[2]));
        let q = crate::picking::closest_on_tri(p, a, b, c);
        let d = (q - p).length_squared();
        if melhor.is_none_or(|x| d < x.0) {
            let Some(mut n) = (b - a).cross(c - a).try_normalize() else { continue };
            if n.dot(vtx(&m.normals, t[0]) + vtx(&m.normals, t[1]) + vtx(&m.normals, t[2])) < 0. {
                n = -n;
            }
            melhor = Some((d, q, n));
        }
    }
    melhor.map(|x| (x.1, x.2))
}

/// Superfície dona da aresta clicada: a de referência ou a face do corpo que tem essa aresta.
pub fn apoio_da_aresta(r: &RecomputeResult, body: usize, edge: usize) -> Option<SurfRef> {
    if let Some(id) = crate::picking::surface_of(r, body) {
        return Some(SurfRef::Ref(id));
    }
    let b = r.bodies.get(body)?;
    let m = &b.mesh;
    let face = m.edge_faces.get(edge)?.iter().copied().find(|f| *f != fenkra_kernel::NO_FACE)? as usize;
    let i0 = *m.face_ranges.get(face)? as usize;
    let vtx = |k: usize| Vec3::new(m.positions[m.indices[k] as usize * 3], m.positions[m.indices[k] as usize * 3 + 1], m.positions[m.indices[k] as usize * 3 + 2]);
    let (p0, p1, p2) = (vtx(i0), vtx(i0 + 1), vtx(i0 + 2));
    let (c, n) = ((p0 + p1 + p2) / 3., (p1 - p0).cross(p2 - p0).try_normalize()?);
    let d = |v: Vec3| [v.x as f64, v.y as f64, v.z as f64];
    Some(SurfRef::Face { point: d(c), normal: d(n), owner: b.face_owner.get(face).copied(), key: b.face_key(face) })
}

/// Ponto e normal da superfície de entrada, o mais perto possível de `perto`.
fn ancora(r: &RecomputeResult, s: &SurfRef, perto: Vec3) -> Option<(Vec3, Vec3)> {
    match s {
        SurfRef::Plane(p) => {
            let (o, n) = crate::handles::plane_of(r, p)?;
            Some((perto - n * (perto - o).dot(n), n))
        }
        SurfRef::Face { point, normal, .. } => Some((v3(*point), v3(*normal).try_normalize()?)),
        SurfRef::Ref(id) => perto_na_malha(&r.surfaces.iter().find(|x| x.id == *id)?.mesh, perto),
    }
}

fn centro(r: &RecomputeResult, s: &SurfRef) -> Option<Vec3> {
    match s {
        SurfRef::Plane(p) => crate::handles::plane_of(r, p).map(|x| x.0),
        SurfRef::Face { point, .. } => Some(v3(*point)),
        SurfRef::Ref(id) => r.surfaces.iter().find(|x| x.id == *id).map(|x| crate::handles::mesh_center(&x.mesh)),
    }
}

/// Onde a normal aparece: a meio caminho entre o centro e a borda, para não cair em cima da seta de
/// lado, que costuma sair do centro.
fn lugar_da_normal(r: &RecomputeResult, s: &SurfRef) -> Option<(Vec3, Vec3)> {
    let c = centro(r, s)?;
    let perto = match s {
        SurfRef::Ref(id) => {
            let m = &r.surfaces.iter().find(|x| x.id == *id)?.mesh;
            let longe = m.positions.chunks_exact(3).map(|q| Vec3::new(q[0], q[1], q[2])).max_by(|a, b| (*a - c).length_squared().total_cmp(&(*b - c).length_squared()))?;
            c + (longe - c) * 0.5
        }
        _ => c,
    };
    ancora(r, s, perto)
}

fn centro_do_corpo(r: &RecomputeResult, doc: &Document, id: Id) -> Option<Vec3> {
    let alvo = doc.feature(id)?.target;
    r.bodies.iter().find(|b| b.collector == alvo).map(|b| crate::handles::mesh_center(&b.mesh))
}

// ---------------------------------------------------------------- setas de lado

/// Seta clicável: `dir` aponta para o lado que fica.
#[derive(Clone, PartialEq, Debug)]
pub struct SetaDeLado {
    /// campo que o clique inverte (ver `inverter_lado`)
    pub campo: u8,
    pub base: Vec3,
    pub dir: Vec3,
    pub rotulo: &'static str,
}

/// O texto da expressão diz que o valor é zero?
fn zerado(e: &str) -> bool {
    e.trim().replace(',', ".").parse::<f64>().is_ok_and(|v| v == 0.)
}

/// Seta de um corte: sai da ferramenta, perto de `cortada`, e aponta para o lado que fica.
fn seta_de_corte(r: &RecomputeResult, campo: u8, tool: &SurfRef, flip: bool, perto: Option<Vec3>, rotulo: &'static str) -> Option<SetaDeLado> {
    let (base, n) = ancora(r, tool, perto.or_else(|| centro(r, tool))?)?;
    Some(SetaDeLado { campo, base, dir: if flip { -n } else { n }, rotulo })
}

/// Toda escolha de lado do comando aberto.
pub fn setas_de_lado(doc: &Document, r: &RecomputeResult, id: Id) -> Vec<SetaDeLado> {
    let mut out = vec![];
    if let Some(rf) = doc.reference(id) {
        match &rf.kind {
            RefKind::SurfSplit { surface, tool, flip, more, more_flip, .. } => {
                let c = centro(r, surface);
                out.extend(seta_de_corte(r, 0, tool, *flip, c, "Fica deste lado"));
                for (i, t) in more.iter().enumerate() {
                    let f = more_flip.get(i).copied().unwrap_or(*flip);
                    out.extend(seta_de_corte(r, 2 + i as u8, t, f, c, "Fica deste lado"));
                }
            }
            RefKind::SurfTrim { a, b, flip_a, flip_b } | RefKind::SurfFillet { a, b, flip_a, flip_b, .. } => {
                out.extend(seta_de_corte(r, 0, b, *flip_a, centro(r, a), "Fica da superfície 1"));
                out.extend(seta_de_corte(r, 1, a, *flip_b, centro(r, b), "Fica da superfície 2"));
            }
            RefKind::CurveParallel { curve, support, flip, .. } => out.extend(seta_da_paralela(r, curve, support, *flip)),
            RefKind::SurfRuled { curves, .. } => {
                // da curva para o meio da faixa: o clique troca o lado
                let base = curves.first().and_then(|c| crate::picking::curve_pts(r, c)).and_then(|p| meio_da_curva(&p)).map(|x| x.0);
                let faixa = r.surfaces.iter().find(|x| x.id == id).map(|x| crate::handles::mesh_center(&x.mesh));
                if let (Some(b), Some(m)) = (base, faixa) {
                    if let Some(dir) = (m - b).try_normalize() {
                        out.push(SetaDeLado { campo: 0, base: b, dir, rotulo: "Sentido da faixa" });
                    }
                }
            }
            _ => {}
        }
        return out;
    }
    let corpo = centro_do_corpo(r, doc, id);
    match doc.feature(id).map(|f| &f.kind) {
        Some(FeatureKind::SplitBody(x)) => out.extend(seta_de_corte(r, 0, &x.surface, x.flip, corpo, "Fica deste lado")),
        Some(FeatureKind::SewSurface(x)) if x.remove => out.extend(seta_de_corte(r, 0, &x.surface, x.flip, corpo, "Fica deste lado")),
        Some(FeatureKind::ThickSurface(t)) => {
            if let Some((base, n)) = centro(r, &t.surface).and_then(|c| ancora(r, &t.surface, c)) {
                if !zerado(&t.t1) {
                    out.push(SetaDeLado { campo: 0, base, dir: n, rotulo: "Espessura 1" });
                }
                if !zerado(&t.t2) {
                    out.push(SetaDeLado { campo: 0, base, dir: -n, rotulo: "Espessura 2" });
                }
            }
        }
        _ => {}
    }
    out
}

/// Ponto na metade do comprimento e a tangente ali. Uma reta tem só dois pontos: o do meio da lista
/// seria a ponta.
fn meio_da_curva(p: &[[f64; 3]]) -> Option<(Vec3, Vec3)> {
    let total: f32 = p.windows(2).map(|w| (v3(w[1]) - v3(w[0])).length()).sum();
    let mut falta = total / 2.;
    for w in p.windows(2) {
        let (a, b) = (v3(w[0]), v3(w[1]));
        let l = (b - a).length();
        if l >= falta && l > 1e-9 {
            return Some((a + (b - a) * (falta / l), (b - a) / l));
        }
        falta -= l;
    }
    None
}

/// A paralela anda para `normal × tangente` a partir da curva, ou o oposto com `flip`.
fn seta_da_paralela(r: &RecomputeResult, curve: &CurveRef, support: &SurfRef, flip: bool) -> Option<SetaDeLado> {
    let (m, t) = meio_da_curva(&crate::picking::curve_pts(r, curve)?)?;
    let (_, n) = ancora(r, support, m)?;
    let lado = n.cross(t).try_normalize()?;
    Some(SetaDeLado { campo: 0, base: m, dir: if flip { -lado } else { lado }, rotulo: "Lado da paralela" })
}

/// Clique na seta: troca o lado. No engrossar, as duas espessuras trocam de lado.
pub fn inverter_lado(doc: &mut Document, id: Id, campo: u8) {
    if let Some(rf) = doc.references.iter_mut().find(|r| r.id == id) {
        match &mut rf.kind {
            RefKind::SurfSplit { flip, more, more_flip, .. } => {
                more_flip.resize(more.len(), *flip);
                match campo {
                    0 => *flip = !*flip,
                    c => {
                        if let Some(f) = more_flip.get_mut(c as usize - 2) {
                            *f = !*f;
                        }
                    }
                }
            }
            RefKind::SurfTrim { flip_a, flip_b, .. } | RefKind::SurfFillet { flip_a, flip_b, .. } => {
                let f = if campo == 0 { flip_a } else { flip_b };
                *f = !*f;
            }
            RefKind::CurveParallel { flip, .. } | RefKind::SurfRuled { flip, .. } => *flip = !*flip,
            _ => {}
        }
        return;
    }
    match doc.feature_mut(id).map(|f| &mut f.kind) {
        Some(FeatureKind::SplitBody(x)) => x.flip = !x.flip,
        Some(FeatureKind::SewSurface(x)) => x.flip = !x.flip,
        Some(FeatureKind::ThickSurface(t)) => std::mem::swap(&mut t.t1, &mut t.t2),
        _ => {}
    }
}

/// Clique na bolinha: fica a solução `k`. Onde há âncora espacial, o ponto da bolinha é a âncora: o
/// número sozinho muda quando o kernel reordena as soluções.
pub fn escolher_solucao(doc: &mut Document, id: Id, k: usize, p: [f64; 3]) {
    let Some(rf) = doc.references.iter_mut().find(|r| r.id == id) else { return };
    match &mut rf.kind {
        RefKind::PointTangent { which, .. } | RefKind::PointCurvePlane { which, .. } => *which = k,
        RefKind::SurfSweepX { solution, .. } => *solution = k,
        RefKind::CurveIntersect { which, near, .. } => {
            *which = k;
            *near = Some(p);
        }
        RefKind::CurveBoundary { which, near, prop, .. } => {
            *which = k;
            *near = Some(p);
            *prop = Propagation::Ponto;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- pedaços (B277)

/// Pedaços do corte por região do comando aberto.
pub fn regioes(r: &RecomputeResult, id: Id) -> &[fenkra_solidos::regioes::RegiaoNota] {
    r.notes.get(&id).map_or(&[], |n| n.regioes.as_slice())
}

/// `sf` é cortada pelo comando `id`? Enquanto ele está aberto, quem aparece são os pedaços.
pub fn entrada_das_regioes(doc: &Document, id: Id, sf: Id) -> bool {
    let Some(RefKind::SurfRegions { surface, tools, mutual, .. }) = doc.reference(id).map(|r| &r.kind) else { return false };
    *surface == SurfRef::Ref(sf) || (*mutual && tools.contains(&SurfRef::Ref(sf)))
}

/// Pedaço sob o ponteiro: o mais perto da câmera, inclusive os que saem, para poderem voltar.
pub fn regiao_sob(r: &RecomputeResult, id: Id, cam: &Camera, rect: Rect, pos: Pos2) -> Option<usize> {
    let (o, d) = cam.ray(rect, pos);
    let mut melhor: Option<(f32, usize)> = None;
    for (i, g) in regioes(r, id).iter().enumerate() {
        let m = &*g.malha;
        let p = |k: u32| Vec3::new(m.positions[k as usize * 3], m.positions[k as usize * 3 + 1], m.positions[k as usize * 3 + 2]);
        for t in m.indices.chunks_exact(3) {
            if let Some(dist) = crate::picking::ray_tri(o, d, p(t[0]), p(t[1]), p(t[2])) {
                if melhor.is_none_or(|x| dist < x.0) {
                    melhor = Some((dist, i));
                }
            }
        }
    }
    melhor.map(|x| x.1)
}

/// Clique no pedaço `i`: sem marca, ganha uma; com marca, perde todas as que caíram nele.
pub fn alternar_regiao(doc: &mut Document, r: &RecomputeResult, id: Id, i: usize) {
    let Some(g) = regioes(r, id).get(i) else { return };
    let Some(rf) = doc.references.iter_mut().find(|x| x.id == id) else { return };
    let RefKind::SurfRegions { marks, .. } = &mut rf.kind else { return };
    if g.marcas.is_empty() {
        marks.push(g.marca());
    } else {
        let mut k = 0;
        marks.retain(|_| {
            k += 1;
            !g.marcas.contains(&(k - 1))
        });
    }
}

/// Opacidade do pedaço que fica: quase cheio, mas ainda deixa ver que é prévia.
const ALFA_FICA: f32 = 0.82;
/// Opacidade do pedaço que sai.
const ALFA_SAI: f32 = 0.16;

fn pintar_regioes(r: &RecomputeResult, f: &crate::previa::Fantasma, s: &mut Scene) {
    for g in regioes(r, f.id) {
        let (cor, alfa) = if g.fica { (f.cor, ALFA_FICA) } else { (f.retirada, ALFA_SAI) };
        crate::previa::malha_fantasma(&mut s.ghost, &g.malha, cor, alfa);
        contorno(&g.malha, crate::previa::cor_linha(cor), s);
    }
}

// ---------------------------------------------------------------- camada 3D

/// O resultado do comando fica em cima da entrada? Então a entrada só tem contorno: duas camadas
/// translúcidas no mesmo lugar brigam pelo pixel.
fn cobre_a_entrada(doc: &Document, id: Id) -> bool {
    use RefKind as R;
    doc.reference(id).is_some_and(|r| {
        matches!(
            r.kind,
            R::SurfSplit { .. } | R::SurfTrim { .. } | R::SurfRegions { .. } | R::SurfFillet { .. } | R::SurfEdgeFillet { .. } | R::SurfHeal { .. } | R::SurfJoin { .. } | R::SurfUntrim { .. } | R::SurfExtrapolate { .. } | R::SurfPart { .. }
        )
    })
}

/// Contorno da malha na camada de traço grosso, que fica por cima de tudo.
fn contorno(m: &Mesh, col: [f32; 4], s: &mut Scene) {
    let nseg = m.edges.len() / 6;
    for e in 0..m.edge_ranges.len() {
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
        if s1 <= s0 {
            continue;
        }
        let mut pts: Vec<[f32; 3]> = m.edges[s0 * 6..s1 * 6].chunks_exact(6).map(|q| [q[0], q[1], q[2]]).collect();
        let f = &m.edges[s1 * 6 - 3..s1 * 6];
        pts.push([f[0], f[1], f[2]]);
        crate::render::push_wide(&mut s.wide, &pts, col, 2.5, 0);
    }
}

/// Realce das entradas na cena, cada uma na cor do seu papel.
pub fn decorar(doc: &Document, r: &RecomputeResult, f: &crate::previa::Fantasma, s: &mut Scene) {
    let f4 = |c: [u8; 3]| c.map(|x| x as f32 / 255.);
    crate::sup_analise::decorar(r, f.id, &[f4(f.cor), f4(f.neutro), f4(f.entrada), f4(f.erro)], s);
    let cobre = cobre_a_entrada(doc, f.id);
    pintar_regioes(r, f, s);
    crate::sup_loft::decorar(doc, r, f, s);
    let tem_regioes = !regioes(r, f.id).is_empty();
    for x in suportes(doc, f.id) {
        // os pedaços já mostram a superfície inteira, com contorno
        if tem_regioes && matches!(x.alvo, Alvo::Sup(SurfRef::Ref(_))) {
            continue;
        }
        let rgb = x.papel.rgb(f.cor);
        let col = crate::previa::cor_linha(rgb);
        match &x.alvo {
            Alvo::Sup(SurfRef::Ref(id)) => {
                let Some(m) = r.surfaces.iter().find(|q| q.id == *id).map(|q| &q.mesh) else { continue };
                // a que segue visível já está opaca: a camada translúcida ficaria em cima dela
                if !cobre && doc.reference(*id).is_some_and(|q| !doc.ref_shown(q)) {
                    (0..m.face_ranges.len()).for_each(|fi| crate::previa::face_fantasma(&mut s.ghost, m, fi, rgb));
                }
                contorno(m, col, s);
            }
            Alvo::Sup(SurfRef::Face { point, .. }) => {
                for b in &r.bodies {
                    if let Some((fi, _)) = crate::picking::face_e_distancia(&b.mesh, *point).filter(|q| q.1 < 0.05) {
                        crate::previa::face_fantasma(&mut s.ghost, &b.mesh, fi, rgb);
                    }
                }
            }
            Alvo::Sup(SurfRef::Plane(p)) | Alvo::Plano(p) => crate::previa::retangulo_do_plano(r, p, rgb, s),
            Alvo::Curva(c) => {
                if let Some(p) = crate::picking::curve_pts(r, c) {
                    let pts: Vec<[f32; 3]> = p.iter().map(|q| v3(*q).to_array()).collect();
                    crate::render::push_wide(&mut s.wide, &pts, col, 3.5, 0);
                }
            }
            Alvo::Ponto(_) => {}
        }
    }
}

/// Comprimento da seta na tela: o tamanho real da peça não importa.
const SETA: f32 = 34.;

/// Base na tela e direção de `dir` projetada. Olhando de frente para a seta, ela aponta para cima.
fn na_tela(rect: Rect, cam: &Camera, base: Vec3, dir: Vec3) -> Option<(Pos2, Vec2)> {
    let s = cam.project(rect, base)?;
    let d = cam.project(rect, base + dir).map(|q| q - s).filter(|v| v.length() > 1e-3).map(|v| v.normalized());
    Some((s, d.unwrap_or(egui::vec2(0., -1.))))
}

fn pintar_seta(p: &egui::Painter, s: Pos2, d: Vec2, comprimento: f32, cor: Color32) {
    let (ponta, n) = (s + d * comprimento, egui::vec2(-d.y, d.x));
    p.line_segment([s, ponta], Stroke::new(2., cor));
    p.add(egui::Shape::convex_polygon(vec![ponta + d * 10., ponta + n * 5.5, ponta - n * 5.5], cor, Stroke::NONE));
}

/// Etiqueta depois da ponta da seta, sempre para o lado de fora: nunca em cima da própria seta.
fn etiqueta(p: &egui::Painter, ui: &egui::Ui, ponta: Pos2, d: Vec2, texto: &str, cor: Color32) {
    let g = p.layout_no_wrap(texto.to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), cor);
    let tam = g.size() + egui::vec2(8., 4.);
    let centro = ponta + d * 14. + egui::vec2(d.x.signum() * tam.x / 2., d.y * tam.y / 2.);
    let caixa = Rect::from_center_size(centro, tam);
    p.rect_filled(caixa, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.9));
    p.galley(caixa.min + egui::vec2(4., 2.), g, cor);
}

/// Seta de sentido no meio da curva e argola no início dela.
fn sentido_da_curva(ui: &egui::Ui, rect: Rect, cam: &Camera, r: &RecomputeResult, c: &CurveRef, cor: Color32) {
    let Some(p) = crate::picking::curve_pts(r, c) else { return };
    let Some((m, t)) = meio_da_curva(&p) else { return };
    let pintor = ui.painter_at(rect);
    if let Some((s, d)) = na_tela(rect, cam, m, t) {
        pintar_seta(&pintor, s, d, 20., cor);
    }
    if let Some(s) = cam.project(rect, v3(p[0])) {
        pintor.circle_stroke(s, 5., Stroke::new(2., cor));
    }
}

/// Setas, normais e bolinhas do comando aberto. Chamado pela viewport antes das alças de valor.
pub fn overlay(app: &mut App, ui: &mut egui::Ui, rect: Rect, cam: &Camera, id: Id) {
    if app.in_asm() {
        return;
    }
    crate::sup_analise::overlay(app, ui, rect, cam, id);
    crate::sup_loft::overlay(app, ui, rect, cam, id);
    crate::sup_spline::overlay(app, ui, rect, cam, id);
    let Some(r) = app.result.as_ref() else { return };
    let doc = &app.open.doc;
    let destaque = crate::theme::accent(ui.visuals());
    let setas = setas_de_lado(doc, r, id);
    let so_setas = matches!(doc.feature(id).map(|f| &f.kind), Some(FeatureKind::ThickSurface(_)));
    let pintor = ui.painter_at(rect);
    for x in suportes(doc, id) {
        let cor = x.papel.cor(destaque);
        match &x.alvo {
            Alvo::Sup(s) => {
                let (tem_seta, ferramenta) = (!setas.is_empty(), x.papel == Papel::Ferramenta);
                if so_setas || (tem_seta && ferramenta) {
                    continue;
                }
                let Some((base, n)) = lugar_da_normal(r, s) else { continue };
                if let Some((q, d)) = na_tela(rect, cam, base, n) {
                    pintor.circle_filled(q, 3., cor);
                    pintar_seta(&pintor, q, d, 26., cor);
                    etiqueta(&pintor, ui, q + d * 26., d, tr("Normal"), cor);
                }
            }
            Alvo::Curva(c) => sentido_da_curva(ui, rect, cam, r, c, cor),
            Alvo::Ponto(p) => {
                if let Some(q) = crate::picking::point_pos(r, p).and_then(|q| cam.project(rect, v3(q))) {
                    pintor.circle_filled(q, 4.5, cor);
                }
            }
            Alvo::Plano(_) => {}
        }
    }
    let mut inverter = None;
    for (i, a) in setas.iter().enumerate() {
        let Some((s, d)) = na_tela(rect, cam, a.base, a.dir) else { continue };
        let centro_da_pega = s + d * (SETA * 0.7);
        let hr = ui.interact(Rect::from_center_size(centro_da_pega, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(("lado_3d", id.0, i)), egui::Sense::click());
        let cor = if hr.hovered() { ui.visuals().strong_text_color() } else { destaque };
        pintor.circle_filled(s, 3.5, cor);
        pintar_seta(&pintor, s, d, SETA, cor);
        etiqueta(&pintor, ui, s + d * SETA, d, tr(a.rotulo), cor);
        if hr.on_hover_text(tr("Clique para inverter o lado.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            inverter = Some(a.campo);
        }
    }
    let nota = r.notes.get(&id);
    let solucoes = nota.map(|n| n.solucoes.clone()).unwrap_or_default();
    let atual = nota.and_then(|n| n.solucao);
    let mut escolha = None;
    for (k, p) in &solucoes {
        let Some(q) = cam.project(rect, v3(*p)) else { continue };
        let hr = ui.interact(Rect::from_center_size(q, egui::vec2(crate::handles::PEGA, crate::handles::PEGA)), egui::Id::unique(("solucao_3d", id.0, *k)), egui::Sense::click());
        let esta = atual == Some(*k);
        let cor = if hr.hovered() { ui.visuals().strong_text_color() } else { destaque };
        if esta {
            pintor.circle_filled(q, 10., cor);
        } else {
            pintor.circle_filled(q, 10., ui.visuals().extreme_bg_color);
        }
        pintor.circle_stroke(q, 10., Stroke::new(2., cor));
        let texto = if esta { ui.visuals().extreme_bg_color } else { cor };
        pintor.text(q, egui::Align2::CENTER_CENTER, format!("{}", k + 1), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), texto);
        if hr.on_hover_text(tr("Clique para ficar com esta solução.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            escolha = Some((*k, *p));
        }
    }
    let mut alternar = None;
    for (i, g) in regioes(r, id).iter().enumerate() {
        let Some(q) = cam.project(rect, v3(g.ponto)) else { continue };
        let (nome, cor) = if g.fica { ("Fica", destaque) } else { ("Sai", crate::theme::papel_retirada()) };
        let t = pintor.layout_no_wrap(tr(nome).to_string(), egui::FontId::proportional(crate::acessibilidade::fonte_3d(12.)), cor);
        let caixa = Rect::from_center_size(q, t.size() + egui::vec2(12., 6.));
        let hr = ui.interact(caixa, egui::Id::unique(("regiao_3d", id.0, i)), egui::Sense::click());
        pintor.rect_filled(caixa, 4., ui.visuals().extreme_bg_color);
        pintor.rect_stroke(caixa, 4., Stroke::new(if hr.hovered() { 2. } else { 1. }, cor), egui::StrokeKind::Inside);
        pintor.galley(caixa.min + egui::vec2(6., 3.), t, cor);
        if hr.on_hover_text(tr("Clique para trocar entre ficar e sair.")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            alternar = Some(i);
        }
    }
    if let Some(i) = alternar {
        let r = app.result.take();
        if let Some(r) = &r {
            alternar_regiao(&mut app.open.doc, r, id, i);
        }
        app.result = r;
    }
    if let Some(c) = inverter {
        inverter_lado(&mut app.open.doc, id, c);
    }
    if let Some((k, p)) = escolha {
        escolher_solucao(&mut app.open.doc, id, k, p);
    }
}

/// Linha do painel no lugar do número da solução.
pub fn dica_de_solucao(ui: &mut egui::Ui, note: Option<&fenkra_solidos::RefNote>) {
    let Some(n) = note.filter(|n| n.solucoes.len() > 1) else { return };
    let atual = n.solucao.map_or(String::from("?"), |k| (k + 1).to_string());
    ui.label(RichText::new(format!("{} {atual} {} {}. {}", tr("Solução"), tr("de"), n.solucoes.len(), tr("Clique na bolinha do 3D para trocar."))).weak().small());
}

/// Linha do painel no lugar da caixa de lado.
pub fn dica_de_lado(ui: &mut egui::Ui) {
    ui.label(RichText::new(tr("A seta do 3D aponta o lado que fica. Clique nela para inverter.")).weak().small());
}

/// Casos do B290: círculo e quadrado com conector, concordância e conectar.
fn demo_b290(nome: &str, doc: &mut Document) -> crate::previa::DemoPrevia {
    use crate::previa::DemoPrevia::Ref;
    let t = |x: &str| x.to_string();
    let fx = PointRef::Fixed;
    let folha = |d: &mut Document, a: [f64; 3], b: [f64; 3]| {
        let c = CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: fx(a), b: fx(b), start: t("0"), end: t("0") }));
        d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Y), length: t("50"), length2: t("0"), until: None, until2: None })
    };
    if nome == "sup_conectores" {
        let z = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "60");
        let _ = z;
        let c = doc.add_geometry(RefKind::Circle { center: fx([0.; 3]), plane: PlaneRef::Origin(OriginPlane::XY), radius: t("25"), start: t("0"), end: t("360") });
        let q = doc.add_geometry(RefKind::Polyline { points: [[20., 20., 60.], [-20., 20., 60.], [-20., -20., 60.], [20., -20., 60.]].map(fx).to_vec(), closed: true });
        return Ref(RefKind::SurfLoft { sections: vec![CurveRef::Ref(c), CurveRef::Ref(q)], guide: None, ruled: false, closed: false, opts: LoftOpts { connectors: vec![vec![0.125, 0.]], iso: 6, tangents: [t("40"), t("0")], ..Default::default() } });
    }
    let a = folha(doc, [-60., 0., 0.], [-20., 0., 0.]);
    let b = folha(doc, [20., 0., 25.], [60., 0., 25.]);
    if nome == "sup_conectar" {
        return Ref(RefKind::SurfConnect { a: SurfRef::Ref(a), b: SurfRef::Ref(b), cont: Continuity::G1, tension: t("1"), tension2: Some(t("0.6")), near: [None; 2] });
    }
    let borda = |x: f64, z: f64| CurveRef::Edge(vec![[x, 0., z], [x, 50., z]]);
    Ref(RefKind::SurfBlend { a: borda(-20., 0.), sa: Some(SurfRef::Ref(a)), b: borda(20., 25.), sb: Some(SurfRef::Ref(b)), cont: Continuity::G2, tension: t("1"), tension2: None })
}

/// Casos do B34: multi-seções com guias, regrada e as análises.
fn demo_b34(nome: &str, doc: &mut Document) -> crate::previa::DemoPrevia {
    use crate::previa::DemoPrevia::Ref;
    let t = |x: &str| x.to_string();
    let fx = PointRef::Fixed;
    let curva = |d: &mut Document, p: &[[f64; 3]]| CurveRef::Ref(d.add_geometry(RefKind::Spline { points: p.iter().map(|q| fx(*q)).collect(), closed: false, tangents: vec![] }));
    let onda = |d: &mut Document, z: f64| {
        let c = curva(d, &[[-40., 0., 0. + z], [0., 12., z], [40., 0., z]]);
        SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Y), length: t("50"), length2: t("0"), until: None, until2: None }))
    };
    match nome {
        "sup_guias" => {
            let s1 = curva(doc, &[[-40., 0., 0.], [0., 10., 0.], [40., 0., 0.]]);
            let s2 = curva(doc, &[[-30., 0., 70.], [0., 16., 70.], [30., 0., 70.]]);
            let g1 = curva(doc, &[[-40., 0., 0.], [-55., 0., 35.], [-30., 0., 70.]]);
            let g2 = curva(doc, &[[40., 0., 0.], [52., 0., 35.], [30., 0., 70.]]);
            Ref(RefKind::SurfLoft { sections: vec![s1, s2], guide: None, ruled: false, closed: false, opts: LoftOpts { guides: vec![g1, g2], ..Default::default() } })
        }
        "sup_regrada" => {
            let s = onda(doc, 0.);
            let borda = curva(doc, &[[-40., 0., 0.], [0., 12., 0.], [40., 0., 0.]]);
            Ref(RefKind::SurfRuled { curves: vec![borda], support: Some(s), mode: RuledMode::Normal, dir: AxisRef::Origin(OriginAxis::Z), length: t("25"), flip: false })
        }
        "sup_conexao" => {
            let a = onda(doc, 0.);
            let c = curva(doc, &[[-40., 0., 0.], [0., 12., 0.], [40., 0., 0.]]);
            let b = SurfRef::Ref(doc.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: t("40"), length2: t("0"), until: None, until2: None }));
            Ref(RefKind::SurfCheck { mode: CheckMode::Conexao, a, b, curve: CurveRef::default(), samples: t("24"), scale: t("1") })
        }
        "sup_desvio" => {
            let a = onda(doc, 0.);
            let c = curva(doc, &[[-40., 0., 6.], [0., 12., 0.], [40., 0., 2.]]);
            let b = SurfRef::Ref(doc.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Y), length: t("50"), length2: t("0"), until: None, until2: None }));
            Ref(RefKind::SurfCheck { mode: CheckMode::Desvio, a, b, curve: CurveRef::default(), samples: t("2"), scale: t("1") })
        }
        _ => {
            let c = curva(doc, &[[-40., 0., 0.], [-15., 20., 0.], [10., -10., 0.], [40., 10., 0.]]);
            Ref(RefKind::SurfCheck { mode: CheckMode::Pente, a: SurfRef::default(), b: SurfRef::default(), curve: c, samples: t("64"), scale: t("400") })
        }
    }
}

/// Desenvolvimento: casos de `FENKRA_DEMO_PREVIA=sup_*`, um por comando, para as capturas.
pub fn demo(nome: &str, doc: &mut Document) -> Option<crate::previa::DemoPrevia> {
    use crate::previa::DemoPrevia::{Comando, Ref};
    if !nome.starts_with("sup_") {
        return None;
    }
    *doc = Document::new("Prévia");
    let t = |x: &str| x.to_string();
    let reta = |d: &mut Document, a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: t("0"), end: t("0") }));
    let folha = |d: &mut Document, a: [f64; 3], b: [f64; 3]| {
        let c = reta(d, a, b);
        SurfRef::Ref(d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: t("30"), length2: t("30"), until: None, until2: None }))
    };
    let yz = SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ));
    Some(match nome {
        "sup_dividir" => {
            let f = folha(doc, [-40., 0., 0.], [40., 0., 0.]);
            Ref(RefKind::SurfSplit { surface: f, tool: yz, flip: false, more: vec![], curve: None, more_flip: vec![] })
        }
        "sup_aparar" | "sup_filete" => {
            let a = folha(doc, [-40., 0., 0.], [40., 0., 0.]);
            let b = folha(doc, [0., -40., 0.], [0., 40., 0.]);
            Ref(if nome == "sup_aparar" { RefKind::SurfTrim { a, b, flip_a: false, flip_b: false } } else { RefKind::SurfFillet { a, b, flip_a: false, flip_b: false, radius: t("6"), radius2: None } })
        }
        "sup_regioes" | "sup_regioes_manter" => {
            let a = folha(doc, [-40., 0., 0.], [40., 0., 0.]);
            let b = folha(doc, [0., -40., 0.], [0., 40., 0.]);
            let marca = |source, point| RegionMark { source, point, sides: vec![] };
            let remove = nome == "sup_regioes";
            Ref(RefKind::SurfRegions { surface: a, tools: vec![b], curve: None, mutual: true, remove, extend: false, marks: vec![marca(0, [-20., 0., 0.]), marca(1, [0., 20., 0.])] })
        }
        "sup_paralela" => {
            let c = reta(doc, [-40., 0., 0.], [40., 0., 0.]);
            Ref(RefKind::CurveParallel { curve: c, support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)), d1: t("12"), d2: t("12"), flip: false, more: vec![], both: false, d_other: String::new() })
        }
        "sup_tangente" => {
            let c = doc.add_geometry(RefKind::Circle { center: PointRef::Fixed([0.; 3]), plane: PlaneRef::Origin(OriginPlane::XY), radius: t("30"), start: t("0"), end: t("360") });
            Ref(RefKind::PointTangent { curve: CurveRef::Ref(c), dir: AxisRef::Origin(OriginAxis::X), which: 0 })
        }
        "sup_intersecao" => {
            let cil = doc.add_geometry(RefKind::SurfCylinder { point: PointRef::Fixed([0., 0., -30.]), dir: AxisRef::Origin(OriginAxis::Z), radius: t("30"), end: t("60"), start: t("0") });
            let p = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "10");
            Ref(RefKind::CurveIntersect { a: SurfRef::Ref(cil), b: SurfRef::Plane(PlaneRef::Datum(p)), which: 0, near: None })
        }
        "sup_varredura" => {
            let g1 = reta(doc, [0., 0., 0.], [0., 0., 80.]);
            let g2 = reta(doc, [40., 0., 0.], [40., 0., 80.]);
            Ref(RefKind::SurfSweepX { kind: SweepKind::CircleRadius { guide1: g1, guide2: g2, radius: t("25") }, spine: None, solution: 0, smooth: false, sections: t("12") })
        }
        "sup_spline" => {
            let p = [[-50., 0., 0.], [-15., 25., 10.], [20., -15., 20.], [55., 10., 5.]].map(PointRef::Fixed).to_vec();
            let vinculo = doc.add_geometry(RefKind::PointCoords { x: t("20"), y: t("-15"), z: t("20"), frame: None });
            let mut pts = p;
            pts[2] = PointRef::Ref(vinculo);
            return Some(Ref(RefKind::Spline { points: pts, closed: false, tangents: vec![None, Some(SplineTan { dir: [0., 0.6, 0.8], tension: 1.5 })] }));
        }
        "sup_conectores" | "sup_concordancia" | "sup_conectar" => return Some(demo_b290(nome, doc)),
        "sup_guias" | "sup_regrada" | "sup_conexao" | "sup_desvio" | "sup_pente" => return Some(demo_b34(nome, doc)),
        "sup_engrossar" => {
            let f = folha(doc, [-40., 0., 0.], [40., 0., 0.]);
            Comando(FeatureKind::ThickSurface(ThickSurface { surface: f, t1: t("6"), t2: t("0"), op: BoolOp::Add }))
        }
        "sup_corpo" => {
            doc.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: t("60"), b: t("40"), c: t("30"), d: t("0"), op: BoolOp::Add }));
            Comando(FeatureKind::SplitBody(SplitBody { surface: yz, flip: false }))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::previa::CorpoBase;
    use crate::scene::{build, SceneOpts};
    use crate::settings::Filter;
    use fenkra_solidos::{recompute, Status};

    fn t(x: &str) -> String {
        x.to_string()
    }

    fn reta(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> CurveRef {
        CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: t("0"), end: t("0") }))
    }

    /// Folha vertical pela reta `a`→`b`, 30 mm para cima e para baixo.
    fn folha(d: &mut Document, a: [f64; 3], b: [f64; 3]) -> Id {
        let c = reta(d, a, b);
        d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: t("30"), length2: t("30"), until: None, until2: None })
    }

    fn fantasma(id: Id, antes: &[CorpoBase]) -> crate::previa::Fantasma<'_> {
        crate::previa::Fantasma { id, cor: [10, 20, 200], entrada: [200, 150, 10], erro: [200, 0, 0], retirada: [150, 0, 150], ferramenta: [0, 200, 200], neutro: [0, 200, 90], antes }
    }

    fn cena(d: &Document, r: &RecomputeResult, id: Id) -> Scene {
        let (sel, filtro) = (crate::app::Sel::None, Filter::default());
        let o = SceneOpts { instancias: &[], forma_oculta: None, editing: None, sel: &sel, filter: &filtro, hidden_env: false, dark_edges: true, tangent_edges: false, hilite: None, hilite_extra: &[], line_w: 1., line_dash: 0, point_style: 0, curvatura: None, fantasma: Some(fantasma(id, &[])) };
        build(d, r, &o)
    }

    fn caixa(r: &RecomputeResult, id: Id) -> ([f32; 3], [f32; 3]) {
        let m = &r.surfaces.iter().find(|s| s.id == id).expect("superfície calculada").mesh;
        let (mut a, mut b) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in m.positions.chunks_exact(3) {
            for k in 0..3 {
                a[k] = a[k].min(p[k]);
                b[k] = b[k].max(p[k]);
            }
        }
        (a, b)
    }

    /// A seta de lado aponta para onde o resultado ficou, e o clique leva o resultado para o outro lado.
    #[test]
    fn seta_do_dividir_aponta_o_lado_que_fica_e_o_clique_inverte() {
        let mut d = Document::new("t");
        let f = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let id = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(f), tool: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)), flip: false, more: vec![], curve: None, more_flip: vec![] });
        for _ in 0..2 {
            let r = recompute(&d);
            assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
            let setas = setas_de_lado(&d, &r, id);
            assert_eq!(setas.len(), 1);
            let (a, b) = caixa(&r, id);
            assert!((b[0] - a[0] - 40.).abs() < 0.01 && (b[2] - a[2] - 60.).abs() < 0.01, "meia folha: {a:?} {b:?}");
            let meio_x = (a[0] + b[0]) / 2.;
            assert!(setas[0].dir.x * meio_x > 0., "a seta {:?} aponta para o pedaço em x={meio_x}", setas[0].dir);
            assert!(setas[0].base.x.abs() < 1e-3, "sai do plano de corte: {:?}", setas[0].base);
            inverter_lado(&mut d, id, setas[0].campo);
        }
    }

    /// Mais de um corte: cada corte tem a sua seta e o clique muda só ele.
    #[test]
    fn cada_corte_tem_a_sua_seta() {
        let mut d = Document::new("t");
        let f = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let p = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "20");
        let id = d.add_geometry(RefKind::SurfSplit {
            surface: SurfRef::Ref(f),
            tool: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)),
            flip: false,
            more: vec![SurfRef::Plane(PlaneRef::Datum(p))],
            curve: None,
            more_flip: vec![true],
        });
        let r = recompute(&d);
        let (a, b) = caixa(&r, id);
        assert!((a[0] - 0.).abs() < 0.01 && (b[0] - 20.).abs() < 0.01, "entre os dois planos: {a:?} {b:?}");
        let s = setas_de_lado(&d, &r, id);
        assert_eq!(s.iter().map(|x| x.campo).collect::<Vec<_>>(), [0, 2]);
        inverter_lado(&mut d, id, 2);
        let r = recompute(&d);
        let (a, b) = caixa(&r, id);
        assert!((a[0] - 20.).abs() < 0.01 && (b[0] - 40.).abs() < 0.01, "depois do segundo corte: {a:?} {b:?}");
    }

    /// Filete e aparar: uma seta por superfície, e cada uma aponta para o pedaço que fica dela.
    #[test]
    fn aparar_e_filete_tem_uma_seta_por_superficie() {
        let mut d = Document::new("t");
        let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let b = folha(&mut d, [0., -40., 0.], [0., 40., 0.]);
        let id = d.add_geometry(RefKind::SurfTrim { a: SurfRef::Ref(a), b: SurfRef::Ref(b), flip_a: false, flip_b: false });
        let r = recompute(&d);
        assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
        let s = setas_de_lado(&d, &r, id);
        assert_eq!(s.len(), 2);
        let (lo, hi) = caixa(&r, id);
        // ficam meia folha em X e meia em Y: as setas dizem de que lado
        assert!(s[0].dir.x.abs() > 0.99 && s[1].dir.y.abs() > 0.99, "{s:?}");
        assert!(if s[0].dir.x > 0. { hi[0] > 39. && lo[0] > -1. } else { lo[0] < -39. && hi[0] < 1. }, "{s:?} {lo:?} {hi:?}");
        assert!(if s[1].dir.y > 0. { hi[1] > 39. && lo[1] > -1. } else { lo[1] < -39. && hi[1] < 1. }, "{s:?} {lo:?} {hi:?}");
    }

    /// Paralela: a seta lateral cai sobre a curva resultado.
    #[test]
    fn seta_da_paralela_aponta_a_curva_nova() {
        let mut d = Document::new("t");
        let c = reta(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let id = d.add_geometry(RefKind::CurveParallel { curve: c, support: SurfRef::Plane(PlaneRef::Origin(OriginPlane::XY)), d1: t("12"), d2: t("12"), flip: false, more: vec![], both: false, d_other: String::new() });
        for _ in 0..2 {
            let r = recompute(&d);
            let s = setas_de_lado(&d, &r, id);
            let y = r.curves.iter().find(|x| x.id == id).expect("curva").pts[0][1] as f32;
            assert!((s[0].dir.y * 12. - y).abs() < 1e-3, "{:?} y={y}", s[0].dir);
            inverter_lado(&mut d, id, 0);
        }
    }

    /// A seta fica na metade do comprimento, também numa reta de dois pontos.
    #[test]
    fn meio_da_curva_e_pelo_comprimento() {
        let (m, t) = meio_da_curva(&[[0., 0., 0.], [10., 0., 0.]]).unwrap();
        assert_eq!((m, t), (Vec3::new(5., 0., 0.), Vec3::X));
        let (m, t) = meio_da_curva(&[[0., 0., 0.], [1., 0., 0.], [1., 9., 0.]]).unwrap();
        assert_eq!((m, t), (Vec3::new(1., 4., 0.), Vec3::Y));
    }

    /// Engrossar: a seta mostra de que lado a espessura cresce e o clique a leva para o outro.
    #[test]
    fn engrossar_inverte_pela_seta() {
        let mut d = Document::new("t");
        let f = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let id = d.add_feature(FeatureKind::ThickSurface(ThickSurface { surface: SurfRef::Ref(f), t1: t("6"), t2: t("0"), op: BoolOp::Add }));
        let y = |d: &Document| {
            let r = recompute(d);
            assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
            let m = &r.bodies[0].mesh;
            let (lo, hi) = m.positions.chunks_exact(3).fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
            let s = setas_de_lado(d, &r, id);
            assert_eq!(s.len(), 1);
            (lo, hi, s[0].dir.y)
        };
        let (lo, hi, dir) = y(&d);
        assert!((hi - lo - 6.).abs() < 0.01, "espessura {lo} {hi}");
        assert!(if dir > 0. { lo.abs() < 0.01 } else { hi.abs() < 0.01 }, "a seta {dir} aponta para o material {lo}..{hi}");
        inverter_lado(&mut d, id, 0);
        let (lo2, hi2, dir2) = y(&d);
        assert!(dir2 * dir < 0. && (lo2 + hi).abs() < 0.01 && (hi2 + lo).abs() < 0.01, "{lo2} {hi2} {dir2}");
    }

    /// Dividir corpo: a seta aponta para a metade que fica.
    #[test]
    fn dividir_corpo_pela_seta() {
        let mut d = Document::new("t");
        d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Caixa, plane: PlaneRef::Origin(OriginPlane::XY), a: t("60"), b: t("40"), c: t("30"), d: t("0"), op: BoolOp::Add }));
        let id = d.add_feature(FeatureKind::SplitBody(SplitBody { surface: SurfRef::Plane(PlaneRef::Origin(OriginPlane::YZ)), flip: false }));
        for _ in 0..2 {
            let r = recompute(&d);
            assert!(matches!(r.status.get(&id), Some(Status::Ok)), "{:?}", r.status.get(&id));
            let m = &r.bodies[0].mesh;
            let x = m.positions.chunks_exact(3).map(|p| p[0]).sum::<f32>() / (m.positions.len() / 3) as f32;
            let s = setas_de_lado(&d, &r, id);
            assert!(s[0].dir.x * x > 0., "{:?} x={x}", s[0].dir);
            inverter_lado(&mut d, id, 0);
        }
    }

    /// Duas soluções na tangente; o clique na bolinha troca o ponto.
    #[test]
    fn bolinha_troca_a_solucao_do_ponto_tangente() {
        let mut d = Document::new("t");
        let c = d.add_geometry(RefKind::Circle { center: PointRef::Fixed([0.; 3]), plane: PlaneRef::Origin(OriginPlane::XY), radius: t("30"), start: t("0"), end: t("360") });
        let id = d.add_geometry(RefKind::PointTangent { curve: CurveRef::Ref(c), dir: AxisRef::Origin(OriginAxis::X), which: 0 });
        let r = recompute(&d);
        let n = r.notes.get(&id).expect("nota");
        assert_eq!((n.solucoes.len(), n.solucao), (2, Some(0)));
        let (k, alvo) = n.solucoes[1];
        escolher_solucao(&mut d, id, k, alvo);
        let r = recompute(&d);
        let p = r.points.iter().find(|x| x.id == id).expect("ponto").p;
        assert!((0..3).all(|i| (p[i] - alvo[i]).abs() < 1e-6), "{p:?} {alvo:?}");
        assert_eq!(r.notes.get(&id).and_then(|n| n.solucao), Some(1));
    }

    /// Interseção com duas curvas: a escolha vira âncora e segue a curva, não o número.
    #[test]
    fn bolinha_grava_a_ancora_da_intersecao() {
        let mut d = Document::new("t");
        let cil = d.add_geometry(RefKind::SurfCylinder { point: PointRef::Fixed([0., 0., -30.]), dir: AxisRef::Origin(OriginAxis::Z), radius: t("30"), end: t("60"), start: t("0") });
        let p = d.add_offset_plane(PlaneRef::Origin(OriginPlane::YZ), "10");
        let id = d.add_geometry(RefKind::CurveIntersect { a: SurfRef::Ref(cil), b: SurfRef::Plane(PlaneRef::Datum(p)), which: 0, near: None });
        let r = recompute(&d);
        let n = r.notes.get(&id).expect("nota");
        assert_eq!(n.solucoes.len(), 2, "o plano corta o cilindro em duas retas");
        let outra = n.solucoes.iter().find(|x| Some(x.0) != n.solucao).copied().expect("a outra");
        escolher_solucao(&mut d, id, outra.0, outra.1);
        let r = recompute(&d);
        let c = r.curves.iter().find(|x| x.id == id).expect("curva");
        assert!((c.pts[0][1] - outra.1[1]).abs() < 1e-6, "{:?} {:?}", c.pts[0], outra.1);
        assert_eq!(r.notes.get(&id).and_then(|n| n.solucao), Some(outra.0));
    }

    /// Varredura com quatro centros: uma bolinha por solução, em lugares diferentes.
    #[test]
    fn varredura_tem_uma_bolinha_por_solucao() {
        let mut d = Document::new("t");
        let g1 = reta(&mut d, [0., 0., 0.], [0., 0., 80.]);
        let g2 = reta(&mut d, [40., 0., 0.], [40., 0., 80.]);
        let id = d.add_geometry(RefKind::SurfSweepX { kind: SweepKind::CircleRadius { guide1: g1, guide2: g2, radius: t("25") }, spine: None, solution: 0, smooth: false, sections: t("12") });
        let r = recompute(&d);
        let n = r.notes.get(&id).expect("nota");
        assert_eq!(n.solucoes.len(), 4, "{n:?}");
        for (i, a) in n.solucoes.iter().enumerate() {
            for b in &n.solucoes[i + 1..] {
                assert!((0..3).any(|k| (a.1[k] - b.1[k]).abs() > 1.), "{a:?} {b:?}");
            }
        }
    }

    /// Duas folhas cruzadas: os pedaços vão para a cena em duas cores e o clique alterna.
    #[test]
    fn pedacos_em_duas_cores_e_clique_alterna() {
        let mut d = Document::new("t");
        let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let b = folha(&mut d, [0., -40., 0.], [0., 40., 0.]);
        let id = d.add_geometry(RefKind::SurfRegions { surface: SurfRef::Ref(a), tools: vec![SurfRef::Ref(b)], curve: None, mutual: true, remove: true, extend: false, marks: vec![] });
        let r = recompute(&d);
        assert_eq!(regioes(&r, id).len(), 4);
        assert!(regioes(&r, id).iter().all(|g| g.fica), "sem clique, tudo fica");
        let area = |r: &RecomputeResult| r.surfaces.iter().find(|s| s.id == id).map(|s| fenkra_solidos::regioes::area_da_malha(&s.mesh));
        assert!((area(&r).unwrap() - 4. * 40. * 60.).abs() < 0.5, "{:?}", area(&r));

        alternar_regiao(&mut d, &r, id, 2);
        let r = recompute(&d);
        assert_eq!(regioes(&r, id).iter().filter(|g| !g.fica).count(), 1);
        assert!((area(&r).unwrap() - 3. * 40. * 60.).abs() < 0.5, "{:?}", area(&r));
        let s = cena(&d, &r, id);
        let tem = |c: [u8; 3], alfa: f32| {
            let c = c.map(|x| x as f32 / 255.);
            s.ghost.iter().filter(|v| (v.col[0] - c[0]).abs() < 1e-3 && (v.col[2] - c[2]).abs() < 1e-3 && (v.col[3] - alfa).abs() < 1e-3).count()
        };
        assert!(tem([10, 20, 200], ALFA_FICA) > 0, "o que fica");
        assert!(tem([150, 0, 150], ALFA_SAI) > 0, "o que sai");
        assert_eq!(s.ghost.len(), regioes(&r, id).iter().map(|g| g.malha.indices.len()).sum::<usize>(), "a superfície inteira não vai por cima dos pedaços");

        // o segundo clique no mesmo pedaço devolve
        let i = regioes(&r, id).iter().position(|g| !g.fica).unwrap();
        alternar_regiao(&mut d, &r, id, i);
        let r = recompute(&d);
        assert!(regioes(&r, id).iter().all(|g| g.fica));
        assert!(matches!(d.reference(id).map(|x| &x.kind), Some(RefKind::SurfRegions { marks, .. }) if marks.is_empty()));
    }

    /// Entradas na cena: a ferramenta e a superfície na cor do papel, a curva com traço grosso.
    #[test]
    fn entradas_aparecem_na_cor_do_papel() {
        let mut d = Document::new("t");
        let a = folha(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let b = folha(&mut d, [0., -40., 0.], [0., 40., 0.]);
        let id = d.add_geometry(RefKind::SurfSplit { surface: SurfRef::Ref(a), tool: SurfRef::Ref(b), flip: false, more: vec![], curve: None, more_flip: vec![] });
        let r = recompute(&d);
        let s = cena(&d, &r, id);
        let cor = |p: Papel| crate::previa::cor_linha(p.rgb([1, 2, 3]));
        let (base, ferr) = (cor(Papel::Base), cor(Papel::Ferramenta));
        assert!(s.wide.iter().any(|v| v.col == base), "contorno da superfície");
        assert!(s.wide.iter().any(|v| v.col == ferr), "contorno da ferramenta");

        let mut d = Document::new("t");
        let c = reta(&mut d, [-40., 0., 0.], [40., 0., 0.]);
        let id = d.add_geometry(RefKind::SurfExtrude { curve: c, dir: AxisRef::Origin(OriginAxis::Z), length: t("30"), length2: t("0"), until: None, until2: None });
        let r = recompute(&d);
        let s = cena(&d, &r, id);
        assert!(s.wide.iter().any(|v| v.col == cor(Papel::Curva)), "curva de entrada");
    }

    /// Aceite: nenhum painel de superfície tem caixa de lado nem campo numérico de solução.
    #[test]
    fn paineis_sem_caixa_de_lado_nem_numero_de_solucao() {
        for (nome, fonte) in [("geo_ui.rs", include_str!("geo_ui.rs")), ("feat_ui.rs", include_str!("feat_ui.rs"))] {
            for proibido in ["checkbox(flip", "checkbox(&mut more_flip", "&mut sp.flip", "&mut sw.flip", "manter o outro lado", "outro lado da 1", "outro lado da 2", "row(ui, \"Solução\"", "Label::new(tr(\"Solução\"))"] {
                assert!(!fonte.contains(proibido), "{nome} ainda tem {proibido}");
            }
        }
    }
}
