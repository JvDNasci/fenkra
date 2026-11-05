// SPDX-License-Identifier: LGPL-2.1-or-later
//! Prévia fantasma do comando aberto. Um comando aberto já está no documento e já é calculado no fio
//! de trás, com o cache do recálculo. A prévia só muda como esse resultado aparece: o que o comando
//! criou sai translúcido na cor de destaque do tema, fora da seleção. Se o cálculo demora, a tela
//! fica com o último resultado, porque `App::result` só troca quando o novo chega.

use crate::app::{App, Session};
use crate::render::{LineVertex, MeshVertex};
use eframe::egui;
use fenkra_core::Id;
use fenkra_kernel::Mesh;
use fenkra_solidos::{BodyResult, RecomputeResult, Status};
use glam::Vec3;

/// Opacidade do fantasma: deixa ver a peça atrás e ainda se destaca do fundo.
const ALFA: f32 = 0.34;

/// Corpo como estava ao abrir o comando.
pub struct CorpoBase {
    pub collector: Id,
    pub mesh: Mesh,
    pub face_owner: Vec<Id>,
}

pub fn corpos_base(r: Option<&RecomputeResult>) -> Vec<CorpoBase> {
    r.map(|r| r.bodies.iter().map(|b| CorpoBase { collector: b.collector, mesh: b.mesh.clone(), face_owner: b.face_owner.clone() }).collect()).unwrap_or_default()
}

/// O que a cena precisa saber da prévia.
pub struct Fantasma<'a> {
    pub id: Id,
    /// cor de destaque do tema
    pub cor: [u8; 3],
    pub antes: &'a [CorpoBase],
}

impl Fantasma<'_> {
    /// Corpo de antes, quando o comando refez este corpo inteiro. Se o corpo é novo, não há nada
    /// para mostrar por baixo.
    pub fn antes_de(&self, b: &BodyResult) -> Option<&CorpoBase> {
        let tudo = !b.face_owner.is_empty() && b.face_owner.iter().all(|w| *w == self.id);
        self.antes.iter().find(|c| tudo && c.collector == b.collector)
    }
}

pub fn da_sessao<'a>(s: Option<&'a Session>, v: &egui::Visuals) -> Option<Fantasma<'a>> {
    let s = s?;
    let c = crate::theme::accent(v);
    Some(Fantasma { id: s.id, cor: [c.r(), c.g(), c.b()], antes: &s.corpos_antes })
}

fn cor_f(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| x as f32 / 255.)
}

/// Acrescenta os triângulos da face `fi` à camada de fantasma.
pub fn face_fantasma(dst: &mut Vec<MeshVertex>, m: &Mesh, fi: usize, cor: [u8; 3]) {
    let c = cor_f(cor);
    let start = m.face_ranges[fi] as usize;
    let end = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    for &vi in &m.indices[start..end] {
        let k = vi as usize * 3;
        dst.push(MeshVertex {
            pos: [m.positions[k], m.positions[k + 1], m.positions[k + 2]],
            nrm: [m.normals[k], m.normals[k + 1], m.normals[k + 2]],
            col: [c[0], c[1], c[2], ALFA],
            mat: [1., 0., 1., 0.],
        });
    }
}

/// A prévia conta no enquadramento: a aba nova não pode ficar fora da tela.
pub fn crescer_com_face(m: &Mesh, fi: usize, cresce: &mut dyn FnMut(Vec3)) {
    let start = m.face_ranges[fi] as usize;
    let end = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    for &vi in &m.indices[start..end] {
        let k = vi as usize * 3;
        cresce(Vec3::new(m.positions[k], m.positions[k + 1], m.positions[k + 2]));
    }
}

/// Arestas de um corpo com fantasma: as que tocam uma face do comando vão na cor da prévia e ficam
/// visíveis em qualquer estilo de exibição; as outras seguem o caminho normal.
#[allow(clippy::too_many_arguments)]
pub fn arestas_com_fantasma(edges: &mut Vec<LineVertex>, lines: &mut Vec<LineVertex>, m: &Mesh, tangent: bool, col: [f32; 4], cor: [u8; 3], owners: &[Id], id: Id) {
    let c = cor_f(cor);
    let c = [c[0], c[1], c[2], 1.];
    let dono = |k: u32| owners.get(k as usize) == Some(&id);
    let nseg = m.edges.len() / 6;
    if m.edge_faces.len() != m.edge_ranges.len() {
        crate::scene::push_edges(edges, m, tangent, col);
        return;
    }
    for (e, [a, b]) in m.edge_faces.iter().enumerate() {
        if !tangent && m.edge_smooth.get(e).is_some_and(|s| *s != 0) {
            continue;
        }
        let (dst, cc) = if dono(*a) || dono(*b) { (&mut *lines, c) } else { (&mut *edges, col) };
        let s0 = m.edge_ranges[e] as usize;
        let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
        for q in m.edges[s0 * 6..s1 * 6].chunks_exact(3) {
            dst.push(LineVertex { pos: [q[0], q[1], q[2]], col: cc });
        }
    }
}

/// Desenvolvimento: `FENKRA_DEMO_PREVIA=ultimo` tira o último comando da peça de exemplo para ele ser
/// reaberto como comando novo assim que a peça calcular (ver `App::demo_previa`).
pub fn demo_preparar(doc: &mut fenkra_core::Document) -> Option<fenkra_core::FeatureKind> {
    let nome = std::env::var("FENKRA_DEMO_PREVIA").ok()?;
    (nome == "ultimo").then(|| doc.features.pop().map(|f| f.kind)).flatten()
}

/// O que a etiqueta da prévia diz agora.
#[derive(Debug, PartialEq)]
pub enum Aviso {
    /// o comando não fecha: a causa vem do cálculo
    Erro(String),
    /// vale, mas tem ressalva, ou ainda falta escolher algo
    Atencao(String),
    /// o cálculo ainda não voltou; a tela segue com o último resultado
    Calculando,
}

pub fn aviso(status: Option<&Status>, calculando: bool) -> Option<Aviso> {
    match status {
        Some(Status::Error(e)) => Some(Aviso::Erro(e.clone())),
        Some(Status::Warning(w)) => Some(Aviso::Atencao(w.clone())),
        _ if calculando => Some(Aviso::Calculando),
        _ => None,
    }
}

/// Etiqueta no topo da viewport com a causa, para quem olha o modelo e não o painel.
pub fn overlay(app: &App, ui: &mut egui::Ui, rect: egui::Rect) {
    let Some(s) = app.session.as_ref() else { return };
    if app.in_asm() {
        return;
    }
    let Some(a) = aviso(app.status_of(s.id), app.calculando()) else { return };
    let (texto, cor) = match &a {
        Aviso::Erro(e) => (format!("{}: {e}", crate::i18n::tr("Prévia inválida")), crate::theme::st_err()),
        Aviso::Atencao(w) => (w.clone(), crate::theme::st_warn()),
        Aviso::Calculando => (crate::i18n::tr("Atualizando a prévia…").to_string(), crate::theme::accent(ui.visuals())),
    };
    let painter = ui.painter_at(rect);
    let g = painter.layout(texto, egui::FontId::proportional(13.), cor, (rect.width() - 260.).clamp(200., 620.));
    let caixa = egui::Rect::from_center_size(rect.center_top() + egui::vec2(0., 62. + g.size().y / 2.), g.size() + egui::vec2(16., 8.));
    painter.rect_filled(caixa, 4., ui.visuals().extreme_bg_color.gamma_multiply(0.92));
    painter.rect_stroke(caixa, 4., egui::Stroke::new(1., cor), egui::StrokeKind::Inside);
    painter.galley(caixa.min + egui::vec2(8., 4.), g, cor);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Scene;
    use crate::scene::{build, SceneOpts};
    use crate::settings::Filter;
    use fenkra_core::*;
    use fenkra_solidos::recompute;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn cena(d: &Document, r: &RecomputeResult, id: Id, antes: &[CorpoBase]) -> Scene {
        let sel = crate::app::Sel::None;
        let filtro = Filter::default();
        let o = SceneOpts {
            editing: None,
            sel: &sel,
            filter: &filtro,
            hidden_env: false,
            dark_edges: true,
            tangent_edges: false,
            hilite: None,
            hilite_extra: &[],
            line_w: 1.,
            line_dash: 0,
            point_style: 0,
            fantasma: Some(Fantasma { id, cor: [10, 20, 30], antes }),
        };
        build(d, r, &o)
    }

    fn sk_rect(d: &mut Document, plano: PlaneRef, a: [f64; 2], b: [f64; 2]) -> Id {
        let c = [a, [b[0], a[1]], b, [a[0], b[1]]];
        sk_linhas(d, plano, &[(c[0], c[1]), (c[1], c[2]), (c[2], c[3]), (c[3], c[0])], true)
    }

    /// Sketch de segmentos; `fechado` emenda os pontos em comum para formar perfil.
    fn sk_linhas(d: &mut Document, plano: PlaneRef, segs: &[([f64; 2], [f64; 2])], fechado: bool) -> Id {
        let mut data = SketchData::default();
        let mut n = 7_000_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        let mut pts: Vec<([f64; 2], Id)> = vec![];
        for (a, b) in segs {
            let mut ponto = |q: [f64; 2], data: &mut SketchData| match pts.iter().find(|(x, _)| fechado && *x == q) {
                Some((_, id)) => *id,
                None => {
                    let id = data.add_point(&mut al, q[0], q[1]);
                    pts.push((q, id));
                    id
                }
            };
            let pa = ponto(*a, &mut data);
            let pb = ponto(*b, &mut data);
            data.add_curve(&mut al, sketch::Curve::Line { a: pa, b: pb }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
    }

    fn sk_pontos(d: &mut Document, plano: PlaneRef, pts: &[[f64; 2]]) -> Id {
        let mut data = SketchData::default();
        let mut n = 7_500_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        for p in pts {
            let id = data.add_point(&mut al, p[0], p[1]);
            data.add_curve(&mut al, sketch::Curve::Point { p: id }, false);
        }
        d.add_feature(FeatureKind::Sketch(Sketch { plane: plano, data }))
    }

    fn chapa() -> Document {
        let mut d = Document::new("chapa");
        d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), flat: false, ..Default::default() });
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [0., 0.], [100., 50.]);
        d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
        d
    }

    fn valido(st: Option<&Status>) -> bool {
        matches!(st, Some(Status::Ok | Status::Warning(_)))
    }

    /// B223: cada comando de chapa, aberto com os valores padrão, aparece no fantasma, translúcido e
    /// na cor pedida, e a peça continua na tela por baixo dele.
    #[test]
    fn todo_comando_de_chapa_aparece_no_fantasma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = chapa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "2");
        let quadrado = sk_rect(&mut d, PlaneRef::Datum(topo), [40., 15.], [60., 35.]);
        let pontos = sk_pontos(&mut d, PlaneRef::Datum(topo), &[[50., 25.]]);
        let perfil = sk_linhas(&mut d, PlaneRef::Origin(OriginPlane::XZ), &[([100., 2.], [100., 20.]), ([100., 20.], [110., 20.])], false);
        let comandos: Vec<(&str, FeatureKind)> = vec![
            ("aba", FeatureKind::SmFlange(SmFlange { edge: [100., 25., 2.], length: s("20"), angle: s("90"), radius: None, off1: s("0"), off2: s("0"), flip: false, relief: None, relief_kind: 0, side: 0, miter: false, min_gap: true, edge_key: None })),
            ("bainha", FeatureKind::SmHem(SmHem { edge: [100., 25., 2.], length: s("8"), gap: s("0"), flip: false, kind: 0, edge_key: None })),
            ("dobra por linha", FeatureKind::SmBend(SmBend { a: [60., 0., 2.], b: [60., 50., 2.], angle: s("90"), radius: None, flip: false, down: false, sketch: None, position: 0 })),
            ("joggle", FeatureKind::SmJoggle(SmJoggle { a: [60., 0., 2.], b: [60., 50., 2.], depth: s("3"), run: s("6"), flip: true, angle: None, plane: None, keep_length: false })),
            ("veneziana", FeatureKind::SmLouver(SmLouver { a: [30., 10., 2.], b: [30., 40., 2.], width: s("12"), height: s("5"), flip: false, down: false })),
            ("estampo", FeatureKind::SmStamp(SmStamp { sketch: quadrado, depth: s("3"), flip: false, kind: 0, width: String::new(), punch: None })),
            ("furo flangeado", FeatureKind::SmFlangedHole(SmFlangedHole { sketch: pontos, diameter: s("8"), height: s("4"), flip: false, points: vec![], standard: String::new() })),
            ("recorte", FeatureKind::SmCut(SmCut { sketch: quadrado, mode: 0, offset: String::new(), draft: s("5"), height: s("4"), flip: false, normal_face: false })),
            ("aba por perfil", FeatureKind::SmProfileFlange(SmProfileFlange { sketch: perfil, l1: s("0"), l2: s("40"), radius: None, flip: false })),
            ("aba curva", FeatureKind::SmCurveFlange(SmCurveFlange { edge: vec![[100., 0., 2.], [100., 25., 2.], [100., 50., 2.]], length: s("10"), flip: false, angle: s("90"), radius: None, side: 0, edge_key: None })),
            ("alívio de canto", FeatureKind::SmCornerRelief(SmCornerRelief { at: [100., 50., 2.], size: s("4"), kind: 0 })),
            ("rasgo circular", FeatureKind::SmArcSlot(SmArcSlot { center: [50., 25., 2.], radius: s("15"), width: s("4"), start: s("0"), sweep: s("90") })),
        ];
        let antes = recompute(&d);
        let base = corpos_base(Some(&antes));
        for (nome, kind) in comandos {
            let mut c = d.clone();
            let id = c.add_feature(kind);
            let r = recompute(&c);
            assert!(valido(r.status.get(&id)), "{nome}: {:?}", r.status.get(&id));
            let cena = cena(&c, &r, id, &base);
            assert!(!cena.ghost.is_empty(), "{nome}: nada no fantasma");
            assert!(cena.ghost.iter().all(|v| v.col[3] < 1. && (v.col[0] - 10. / 255.).abs() < 1e-6), "{nome}: fora da cor da prévia");
            assert!(!cena.indices.is_empty(), "{nome}: a peça sumiu de baixo do fantasma");
        }
    }

    /// O fantasma não é selecionável: o raio passa pela aba e acerta o que está atrás, ou nada.
    #[test]
    fn fantasma_nao_entra_na_selecao() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = chapa();
        let aba = d.add_feature(FeatureKind::SmFlange(serde_json::from_value(serde_json::json!({
            "edge": [100., 25., 2.], "length": "30", "angle": "90", "radius": null, "off1": "0", "off2": "0", "flip": false
        })).expect("aba")));
        let r = recompute(&d);
        assert!(valido(r.status.get(&aba)), "{:?}", r.status.get(&aba));
        let b = &r.bodies[0];
        let i = b.face_owner.iter().position(|w| *w == aba).expect("face da aba");
        let m = &b.mesh;
        let ponto = |k: u32| Vec3::new(m.positions[k as usize * 3], m.positions[k as usize * 3 + 1], m.positions[k as usize * 3 + 2]);
        let t0 = m.face_ranges[i] as usize;
        let alvo = (ponto(m.indices[t0]) + ponto(m.indices[t0 + 1]) + ponto(m.indices[t0 + 2])) / 3.;
        let rect = egui::Rect::from_min_size(egui::pos2(0., 0.), egui::vec2(800., 600.));
        let cam = crate::render::Camera { target: alvo, yaw: 0.3, pitch: 0.2, dist: 200., ortho: true };
        let pos = cam.project(rect, alvo).expect("na tela");
        let filtro = |fantasma| crate::picking::Filter { faces: true, edges: false, vertices: false, planes: false, hidden_env: false, exclude_sketch: None, fantasma };
        let dono = |p: Option<crate::picking::Pick>| match p {
            Some(crate::picking::Pick::Face { owner, .. }) => Some(owner),
            _ => None,
        };
        assert_eq!(dono(crate::picking::pick(&r, &d, &cam, rect, pos, &filtro(None))), Some(aba), "sem prévia a aba é uma face comum");
        assert_ne!(dono(crate::picking::pick(&r, &d, &cam, rect, pos, &filtro(Some(aba)))), Some(aba), "com prévia ela não é selecionável");
    }

    /// A causa do erro chega ao texto; enquanto calcula, a etiqueta avisa e a cena fica como está.
    #[test]
    fn aviso_traz_a_causa_e_o_erro_vale_mais_que_calcular() {
        assert_eq!(aviso(Some(&Status::Error(s("colide com a aba vizinha"))), true), Some(Aviso::Erro(s("colide com a aba vizinha"))));
        assert_eq!(aviso(Some(&Status::Warning(s("escolha a aresta"))), false), Some(Aviso::Atencao(s("escolha a aresta"))));
        assert_eq!(aviso(Some(&Status::Ok), true), Some(Aviso::Calculando));
        assert_eq!(aviso(Some(&Status::Ok), false), None);
    }
}
