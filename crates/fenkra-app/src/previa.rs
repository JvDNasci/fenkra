// SPDX-License-Identifier: LGPL-2.1-or-later
//! Prévia fantasma do comando aberto. Um comando aberto já está no documento e já é calculado no fio
//! de trás, com o cache do recálculo. A prévia só muda como esse resultado aparece: o que o comando
//! criou sai translúcido na cor de destaque do tema, fora da seleção. Se o cálculo demora, a tela
//! fica com o último resultado, porque `App::result` só troca quando o novo chega.

use crate::app::{App, Session};
use crate::render::{LineVertex, MeshVertex};
use eframe::egui;
use fenkra_core::{Document, FeatureKind, Id};
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
    /// faces que o comando escolheu (abertas na casca, a face da rosca): cor própria do tema
    pub entrada: [u8; 3],
    /// as mesmas faces quando o comando falhou: é ali que a pessoa precisa olhar
    pub erro: [u8; 3],
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
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    Some(Fantasma { id: s.id, cor: rgb(c), entrada: rgb(crate::theme::st_warn()), erro: rgb(crate::theme::st_err()), antes: &s.corpos_antes })
}

fn cor_f(c: [u8; 3]) -> [f32; 3] {
    c.map(|x| x as f32 / 255.)
}

pub fn cor_linha(c: [u8; 3]) -> [f32; 4] {
    let f = cor_f(c);
    [f[0], f[1], f[2], 1.]
}

/// Malha onde as faces escolhidas pelo comando moram: a de antes, porque a de agora pode já não ter
/// a face (a casca abre, a espessura desloca).
fn malha_de_entrada<'a>(r: &'a RecomputeResult, f: &'a Fantasma, alvo: Id) -> Option<&'a Mesh> {
    f.antes.iter().find(|c| c.collector == alvo).map(|c| &c.mesh).or_else(|| r.bodies.iter().find(|b| b.collector == alvo).map(|b| &b.mesh))
}

/// Tolerância para o ponto guardado cair sobre a face (mm).
const SOBRE_A_FACE: f32 = 0.05;

/// O que cada comando mostra além do resultado: as faces escolhidas, em cor própria, e a hélice da
/// rosca. Com erro, as faces escolhidas ficam na cor de erro: mostra onde falhou em vez de só sumir.
pub fn decorar(doc: &Document, r: &RecomputeResult, f: &Fantasma, s: &mut crate::render::Scene) {
    let Some(feat) = doc.feature(f.id) else { return };
    let falhou = matches!(r.status.get(&f.id), Some(Status::Error(_)));
    if let Some(m) = malha_de_entrada(r, f, feat.target) {
        for p in crate::feat_ui::face_marks(&feat.kind).into_iter().filter(|p| *p != [0.; 3]) {
            if let Some((fi, _)) = crate::picking::face_e_distancia(m, p).filter(|x| x.1 < SOBRE_A_FACE) {
                face_fantasma(&mut s.ghost, m, fi, if falhou { f.erro } else { f.entrada });
            }
        }
    }
    if matches!(feat.kind, FeatureKind::Thread(_)) && !falhou {
        if let Some(h) = r.roscas.iter().find(|x| x.feature == f.id) {
            helice(h, cor_linha(f.cor), cor_linha(f.entrada), &mut s.overlay);
        }
    }
}

/// Base perpendicular a um eixo unitário.
fn base_do_eixo(d: Vec3) -> (Vec3, Vec3) {
    let u = d.any_orthonormal_vector();
    (u, d.cross(u))
}

/// Pontos da hélice simplificada: 24 por volta, no máximo 80 voltas (o desenho é um aviso, não a
/// rosca real).
pub fn pontos_da_helice(h: &fenkra_solidos::RoscaViz) -> Vec<Vec3> {
    let (o, d) = (Vec3::from(h.origin.map(|x| x as f32)), Vec3::from(h.dir.map(|x| x as f32)));
    let (u, v) = base_do_eixo(d);
    let voltas = (h.len / h.pitch).clamp(1., 80.) as f32;
    let n = (voltas * 24.) as usize;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let a = t * voltas * std::f32::consts::TAU;
            o + d * (t * h.len as f32) + (u * a.cos() + v * a.sin()) * h.radius as f32
        })
        .collect()
}

/// Hélice e os anéis de início e fim, sempre visíveis: em furo a rosca fica dentro da peça.
fn helice(h: &fenkra_solidos::RoscaViz, cor: [f32; 4], anel: [f32; 4], dst: &mut Vec<LineVertex>) {
    let pts = pontos_da_helice(h);
    for w in pts.windows(2) {
        dst.push(LineVertex { pos: w[0].into(), col: cor });
        dst.push(LineVertex { pos: w[1].into(), col: cor });
    }
    let (o, d) = (Vec3::from(h.origin.map(|x| x as f32)), Vec3::from(h.dir.map(|x| x as f32)));
    let (u, v) = base_do_eixo(d);
    for c in [o, o + d * h.len as f32] {
        for i in 0..48 {
            let p = |k: i32| {
                let a = k as f32 / 48. * std::f32::consts::TAU;
                c + (u * a.cos() + v * a.sin()) * h.radius as f32
            };
            dst.push(LineVertex { pos: p(i).into(), col: anel });
            dst.push(LineVertex { pos: p(i + 1).into(), col: anel });
        }
    }
}

/// Plano de corte da prévia da casca: passa pelo centro da peça, normal ao eixo que mais aponta para
/// a câmera, e tira a metade da frente. Assim a parede aparece de qualquer ângulo.
pub fn plano_de_secao(limites: (Vec3, Vec3), olho: Vec3) -> [f32; 4] {
    let c = (limites.0 + limites.1) / 2.;
    let v = olho - c;
    let a = v.abs();
    let n = if a.x >= a.y && a.x >= a.z {
        Vec3::X * v.x.signum()
    } else if a.y >= a.z {
        Vec3::Y * v.y.signum()
    } else {
        Vec3::Z * v.z.signum()
    };
    [n.x, n.y, n.z, n.dot(c)]
}

/// Chave da opção "corte de seção na prévia" (vale enquanto o app está aberto).
pub fn chave_secao() -> egui::Id {
    egui::Id::unique("casca_secao_previa")
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

/// Comando que o modo de demonstração abre quando a peça terminar de calcular.
pub enum DemoPrevia {
    Comando(FeatureKind),
    /// plano deslocado 30 mm do plano XY
    Plano,
}

/// Desenvolvimento: `FENKRA_DEMO_PREVIA=<nome>` monta o caso e devolve o comando a abrir como novo
/// (ver `App::demo_previa`). `ultimo` e `membro` usam a peça de exemplo; os outros criam a sua.
pub fn demo_preparar(doc: &mut Document) -> Option<DemoPrevia> {
    use fenkra_core::*;
    let nome = std::env::var("FENKRA_DEMO_PREVIA").ok()?;
    let s = |x: &str| x.to_string();
    let sketch = |d: &mut Document, plane: PlaneRef, monta: &dyn Fn(&mut SketchData, &mut dyn FnMut() -> Id)| {
        let mut data = SketchData::default();
        let mut n = 9_500_000u64 + d.features.len() as u64 * 1000;
        let mut al = || {
            n += 1;
            Id(n)
        };
        monta(&mut data, &mut al);
        d.add_feature(FeatureKind::Sketch(Sketch { plane, data }))
    };
    let caixa = |d: &mut Document| {
        let sk = sketch(d, PlaneRef::Origin(OriginPlane::XY), &|data, al| {
            data.add_rect(&mut &mut *al, [-30., -20.], [30., 20.], false);
        });
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("30")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None }));
    };
    // os casos próprios partem do zero: a placa de exemplo faria a casca falhar
    if matches!(nome.as_str(), "casca" | "espessura" | "rosca" | "nervura") {
        *doc = Document::new("Prévia");
    }
    Some(DemoPrevia::Comando(match nome.as_str() {
        "ultimo" => doc.features.pop()?.kind,
        "membro" => {
            doc.features.pop();
            doc.features.pop()?.kind
        }
        "plano" => return Some(DemoPrevia::Plano),
        "casca" => {
            caixa(doc);
            FeatureKind::Shell(Shell { faces: vec![[0., 0., 30.]], thickness: s("3"), outward: false, face_keys: vec![], closed: false })
        }
        "espessura" => {
            caixa(doc);
            FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 30.]], value: s("8"), face_keys: vec![], remove: false })
        }
        "rosca" => {
            doc.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s("6"), b: s("40"), c: s("0"), d: s("0"), op: BoolOp::Add }));
            FeatureKind::Thread(Thread { face: [6., 0., 20.], spec: s("M10x1.5"), length: s("20"), modeled: false, end: None, start: s("6") })
        }
        "nervura" => {
            caixa(doc);
            let topo = doc.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "30");
            let sk = sketch(doc, PlaneRef::Datum(topo), &|data, al| {
                let a = data.add_point(&mut &mut *al, -25., 0.);
                let b = data.add_point(&mut &mut *al, 25., 0.);
                data.add_curve(&mut &mut *al, fenkra_core::sketch::Curve::Line { a, b }, false);
            });
            FeatureKind::Stiffener(Stiffener { sketch: sk, thickness: s("4"), depth: s("15"), reverse: true, to_body: false, side: 1 })
        }
        _ => return None,
    }))
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

/// Rótulo pequeno preso a um ponto da tela.
fn rotulo(ui: &egui::Ui, rect: egui::Rect, at: egui::Pos2, texto: String, cor: egui::Color32) {
    let painter = ui.painter_at(rect);
    let g = painter.layout_no_wrap(texto, egui::FontId::proportional(12.), cor);
    let caixa = egui::Rect::from_min_size(at + egui::vec2(10., -10.), g.size() + egui::vec2(8., 4.));
    painter.rect_filled(caixa, 3., ui.visuals().extreme_bg_color.gamma_multiply(0.9));
    painter.galley(caixa.min + egui::vec2(4., 2.), g, cor);
}

/// Rótulos que dependem do comando: início e fim da rosca, lado da parede da nervura.
fn rotulos(app: &App, ui: &egui::Ui, rect: egui::Rect, cam: &crate::render::Camera, id: Id) {
    let (Some(f), Some(r)) = (app.open.doc.feature(id), app.result.as_ref()) else { return };
    let cor = crate::theme::accent(ui.visuals());
    match &f.kind {
        FeatureKind::Thread(_) => {
            let Some(h) = r.roscas.iter().find(|x| x.feature == id) else { return };
            let (o, d) = (Vec3::from(h.origin.map(|x| x as f32)), Vec3::from(h.dir.map(|x| x as f32)));
            let borda = base_do_eixo(d).0 * h.radius as f32;
            for (p, nome) in [(o + borda, "Início"), (o + borda + d * h.len as f32, "Fim")] {
                if let Some(q) = cam.project(rect, p) {
                    rotulo(ui, rect, q, crate::i18n::tr(nome).to_string(), crate::theme::st_warn());
                }
            }
        }
        FeatureKind::Stiffener(st) => {
            let Some(sk) = r.sketches.iter().find(|s| s.feature == st.sketch) else { return };
            let t = fenkra_core::expr::eval_str(&st.thickness, &app.vars()).unwrap_or(0.);
            let n0 = Vec3::from(sk.frame.normal.map(|x| x as f32));
            for (i, par) in sk.lines.chunks_exact(2).take(12).enumerate() {
                let (a, b) = (Vec3::from(par[0]), Vec3::from(par[1]));
                let Some(lado) = (b - a).try_normalize().map(|d| n0.cross(d)) else { continue };
                let m = (a + b) / 2.;
                let Some(sm) = cam.project(rect, m) else { continue };
                for sgn in sentidos_da_parede(st.side) {
                    seta_da_parede(ui, rect, cam, m, sm, lado * sgn, cor);
                }
                if i == 0 {
                    let nome = ["dois lados", "um lado", "outro lado"][st.side.min(2) as usize];
                    let extensao = if st.to_body { format!(" · {}", crate::i18n::tr("até o corpo")) } else { String::new() };
                    rotulo(ui, rect, sm, format!("{} {} · {}{extensao}", crate::app::fmt(t), "mm", crate::i18n::tr(nome)), cor);
                }
            }
        }
        _ => {}
    }
}

/// Para onde a parede da nervura cresce a partir da linha, na convenção do cálculo: o lado 1 é o da
/// normal do plano vezes a linha; o lado 2 é o oposto; o 0 vai para os dois.
pub fn sentidos_da_parede(lado: u8) -> &'static [f32] {
    match lado {
        1 => &[1.],
        2 => &[-1.],
        _ => &[1., -1.],
    }
}

/// Seta de tamanho fixo na tela: a espessura real pode ter menos de um pixel no zoom atual.
fn seta_da_parede(ui: &egui::Ui, rect: egui::Rect, cam: &crate::render::Camera, m: Vec3, sm: egui::Pos2, dir: Vec3, cor: egui::Color32) {
    let Some(d) = cam.project(rect, m + dir).map(|q| q - sm).filter(|v| v.length() > 1e-3).map(|v| v.normalized()) else { return };
    let (ponta, n) = (sm + d * 30., egui::vec2(-d.y, d.x));
    let painter = ui.painter_at(rect);
    painter.line_segment([sm, ponta], egui::Stroke::new(2., cor));
    painter.add(egui::Shape::convex_polygon(vec![ponta + d * 9., ponta + n * 5., ponta - n * 5.], cor, egui::Stroke::NONE));
}

/// Etiqueta no topo da viewport com a causa, para quem olha o modelo e não o painel.
pub fn overlay(app: &App, ui: &mut egui::Ui, rect: egui::Rect, cam: &crate::render::Camera) {
    let Some(s) = app.session.as_ref() else { return };
    if app.in_asm() {
        return;
    }
    rotulos(app, ui, rect, cam, s.id);
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
            fantasma: Some(Fantasma { id, cor: [10, 20, 30], entrada: [40, 50, 60], erro: [70, 80, 90], antes }),
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

    fn caixa() -> Document {
        let mut d = Document::new("caixa");
        let sk = sk_rect(&mut d, PlaneRef::Origin(OriginPlane::XY), [-20., -20.], [20., 20.]);
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind(s("20")), reverse: false, taper: s("0"), thin: None, thin_side: 0, dir: None }));
        d
    }

    /// Cor da face na camada de fantasma, em u8.
    fn tem_cor(c: &Scene, cor: [u8; 3]) -> bool {
        c.ghost.iter().any(|v| (0..3).all(|i| (v.col[i] - cor[i] as f32 / 255.).abs() < 1e-6))
    }

    /// B06: a face aberta da casca sai na cor própria; se a casca falha, a mesma face sai na cor de erro.
    #[test]
    fn casca_mostra_a_face_aberta_e_a_regiao_do_erro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let antes = recompute(&d);
        let base = corpos_base(Some(&antes));
        let casca = d.add_feature(FeatureKind::Shell(Shell { faces: vec![[0., 0., 20.]], thickness: s("2"), outward: false, face_keys: vec![], closed: false }));
        let r = recompute(&d);
        assert!(valido(r.status.get(&casca)), "{:?}", r.status.get(&casca));
        let v = r.bodies[0].mass.volume;
        assert!(v < 40. * 40. * 20. * 0.5, "a casca tira material: {v}");
        let c = cena(&d, &r, casca, &base);
        assert!(tem_cor(&c, [40, 50, 60]), "face aberta na cor de entrada");
        assert!(tem_cor(&c, [10, 20, 30]), "paredes novas na cor da prévia");
        // expressão inválida: o cálculo falha e a face escolhida vira a marca do erro
        if let Some(FeatureKind::Shell(sh)) = d.feature_mut(casca).map(|f| &mut f.kind) {
            sh.thickness = s("nao_existe");
        }
        let r = recompute(&d);
        assert!(matches!(r.status.get(&casca), Some(Status::Error(_))), "{:?}", r.status.get(&casca));
        let c = cena(&d, &r, casca, &base);
        assert!(tem_cor(&c, [70, 80, 90]), "a face do comando que falhou fica na cor de erro");
        assert!(!tem_cor(&c, [40, 50, 60]));
    }

    #[test]
    fn plano_de_secao_tira_a_metade_da_frente() {
        let p = plano_de_secao((Vec3::ZERO, Vec3::splat(10.)), Vec3::new(5., -100., 6.));
        assert_eq!(p, [0., -1., 0., -5.]);
        // ponto perto do olho fica do lado cortado (positivo)
        assert!(Vec3::new(p[0], p[1], p[2]).dot(Vec3::new(5., -1., 5.)) > p[3]);
    }

    /// B05: a hélice começa e termina onde o cálculo disse, e o raio é o da face.
    #[test]
    fn helice_liga_inicio_e_fim_da_rosca() {
        let h = fenkra_solidos::RoscaViz { feature: Id(1), origin: [0., 0., 5.], dir: [0., 0., 1.], radius: 4., len: 10., pitch: 1.25 };
        let p = pontos_da_helice(&h);
        assert_eq!(p.len(), 8 * 24 + 1);
        assert!((p[0].z - 5.).abs() < 1e-4 && (p.last().unwrap().z - 15.).abs() < 1e-4);
        assert!(p.iter().all(|q| (q.truncate().length() - 4.).abs() < 1e-3));
    }

    /// B05: rosca cosmética em cilindro: a face escolhida realça, a hélice aparece e a medida errada avisa.
    #[test]
    fn rosca_realca_a_face_e_avisa_o_diametro() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("eixo");
        let pr = d.add_feature(FeatureKind::Primitive(Primitive { kind: PrimKind::Cilindro, plane: PlaneRef::Origin(OriginPlane::XY), a: s("5"), b: s("30"), c: s("0"), d: s("0"), op: BoolOp::Add }));
        let antes = recompute(&d);
        assert!(valido(antes.status.get(&pr)), "{:?}", antes.status.get(&pr));
        let base = corpos_base(Some(&antes));
        let th = d.add_feature(FeatureKind::Thread(Thread { face: [5., 0., 15.], spec: s("M12x1.75"), length: s("10"), modeled: false, end: None, start: s("4") }));
        let r = recompute(&d);
        assert!(matches!(r.status.get(&th), Some(Status::Warning(w)) if w.contains('≠')), "{:?}", r.status.get(&th));
        let c = cena(&d, &r, th, &base);
        assert!(tem_cor(&c, [40, 50, 60]), "a face escolhida realça");
        let h = r.roscas.iter().find(|x| x.feature == th).expect("posição da rosca");
        assert!((h.radius - 5.).abs() < 1e-6 && (h.len - 10.).abs() < 1e-6);
        assert!(c.overlay.len() > 200, "hélice e anéis");
    }

    /// B07: espessura em face: o acréscimo vai para o fantasma e a face escolhida realça.
    #[test]
    fn espessura_em_face_aparece_no_fantasma() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let antes = recompute(&d);
        let base = corpos_base(Some(&antes));
        let e = d.add_feature(FeatureKind::Thickness(Thickness { faces: vec![[0., 0., 20.]], value: s("5"), face_keys: vec![], remove: false }));
        let r = recompute(&d);
        assert!(valido(r.status.get(&e)), "{:?}", r.status.get(&e));
        assert!((r.bodies[0].mass.volume - 40. * 40. * 25.).abs() < 1., "{}", r.bodies[0].mass.volume);
        let c = cena(&d, &r, e, &base);
        assert!(tem_cor(&c, [10, 20, 30]) && tem_cor(&c, [40, 50, 60]));
        assert!(c.bounds.is_some_and(|(_, hi)| (hi.z - 25.).abs() < 1e-3), "a prévia entra no enquadramento");
    }

    /// B11: a seta segue a convenção do cálculo.
    #[test]
    fn nervura_de_um_lado_cresce_para_o_lado_da_seta() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = caixa();
        let topo = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "20");
        let sk = sk_linhas(&mut d, PlaneRef::Datum(topo), &[([-20., 0.], [20., 0.])], false);
        d.add_feature(FeatureKind::Stiffener(Stiffener { sketch: sk, thickness: s("4"), depth: s("5"), reverse: true, to_body: false, side: 1 }));
        let r = recompute(&d);
        // normal +Z vezes linha +X dá +Y: a parede fica em y de 0 a 4
        let ys: Vec<f32> = r.bodies[0].mesh.positions.chunks_exact(3).filter(|p| p[2] > 20.5).map(|p| p[1]).collect();
        assert!(!ys.is_empty() && ys.iter().all(|y| *y > -1e-3 && *y < 4.001), "{ys:?}");
        assert_eq!(sentidos_da_parede(1), &[1.]);
        assert_eq!(sentidos_da_parede(2), &[-1.]);
        assert_eq!(sentidos_da_parede(0).len(), 2);
    }

    fn membro(paths: Vec<CurveRef>) -> StructMember {
        StructMember {
            paths,
            family: s("Tubo quadrado"),
            designation: s("40x3"),
            kind: SectionKind::TuboQuadrado,
            dims: vec![s("40"), s("3")],
            anchor: 0,
            angle: s("0"),
            off_x: s("0"),
            off_y: s("0"),
            ext1: s("0"),
            ext2: s("0"),
            corner: CornerMode::MeiaEsquadria,
            gap: s("0"),
            joints: vec![],
            material: String::new(),
        }
    }

    /// B226: membro sobre um esqueleto em L: o perfil aparece translúcido sobre as duas linhas.
    #[test]
    fn membro_mostra_o_perfil_sobre_cada_linha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let mut d = Document::new("quadro");
        let mut linha = |a: [f64; 3], b: [f64; 3]| CurveRef::Ref(d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(a), b: PointRef::Fixed(b), start: s("0"), end: s("0") }));
        let paths = vec![linha([0., 0., 0.], [400., 0., 0.]), linha([400., 0., 0.], [400., 300., 0.])];
        let m = d.add_feature(FeatureKind::StructMember(membro(paths)));
        let r = recompute(&d);
        assert!(valido(r.status.get(&m)), "{:?}", r.status.get(&m));
        assert!(r.bodies[0].mass.volume > 0.);
        let c = cena(&d, &r, m, &[]);
        assert!(c.indices.is_empty() && !c.ghost.is_empty(), "antes de confirmar o membro é só fantasma");
        let (lo, hi) = c.bounds.expect("caixa");
        assert!(hi.x - lo.x > 390. && hi.y - lo.y > 290., "o fantasma cobre as duas linhas: {lo} {hi}");
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
