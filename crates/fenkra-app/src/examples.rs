//! Peças de exemplo da tela inicial (com prévia 3D girando).

use crate::settings::Bench;
use eframe::egui;
use fenkra_core::sketch::Curve;
use fenkra_core::*;

pub struct Example {
    pub name: &'static str,
    pub desc: &'static str,
    pub bench: Bench,
    pub doc: Document,
}

fn ids(base: u64) -> impl FnMut() -> Id {
    let mut n = base;
    move || {
        n += 1;
        Id(n)
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

fn ext(sketch: Id, d: &str) -> FeatureKind {
    FeatureKind::Extrude(Extrude { sketch, op: BoolOp::Add, extent: Extent::Blind(d.into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0 })
}

fn flange() -> Document {
    let mut d = Document::new("Flange");
    let mut data = SketchData::default();
    let mut al = ids(11_000_000);
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Circle { c, r: 45. }, false);
    data.add_curve(&mut al, Curve::Circle { c, r: 16. }, false);
    for k in 0..6 {
        let a = k as f64 / 6. * std::f64::consts::TAU;
        let p = data.add_point(&mut al, 32. * a.cos(), 32. * a.sin());
        data.add_curve(&mut al, Curve::Circle { c: p, r: 4. }, false);
    }
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(ext(sk, "10"));
    // cubo central
    let top = d.add_offset_plane(PlaneRef::Origin(OriginPlane::XY), "10");
    let mut data = SketchData::default();
    let mut al = ids(11_100_000);
    let c = data.add_point(&mut al, 0., 0.);
    data.add_curve(&mut al, Curve::Circle { c, r: 24. }, false);
    data.add_curve(&mut al, Curve::Circle { c, r: 16. }, false);
    let sk2 = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Datum(top), data }));
    d.add_feature(ext(sk2, "18"));
    if let Some(r) = d.references.iter_mut().find(|r| r.id == top) {
        r.visible = false;
    }
    d
}

fn caixa() -> Document {
    let mut d = Document::new("Caixa com casca");
    let mut data = SketchData::default();
    let mut al = ids(12_000_000);
    data.add_slot(&mut al, [-30., 0.], [30., 0.], 25.);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(ext(sk, "35"));
    d.add_feature(FeatureKind::Shell(Shell { faces: vec![[0., 0., 35.]], thickness: s("2.5"), outward: false }));
    d
}

fn suporte() -> Document {
    let mut d = Document::new("Suporte de chapa");
    d.sheet = Some(SheetParams { thickness: s("2"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut al = ids(13_000_000);
    data.add_rect(&mut al, [0., 0.], [100., 60.], false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    d.add_feature(FeatureKind::SmFlange(SmFlange { edge: [100., 30., 2.], length: s("25"), angle: s("90"), radius: None, off1: s("8"), off2: s("8"), flip: false, relief: Some(s("3")), relief_kind: 1, side: 0, miter: false, min_gap: true }));
    d.add_feature(FeatureKind::SmHem(SmHem { edge: [0., 30., 2.], length: s("8"), gap: s("0"), flip: false, kind: 0 }));
    d.add_feature(FeatureKind::SmLouver(SmLouver { a: [60., 15., 2.], b: [60., 45., 2.], width: s("12"), height: s("5"), flip: false, down: false }));
    d.hide_consumed(d.features[1].id);
    d
}

fn bandeja() -> Document {
    let mut d = Document::new("Bandeja curva");
    d.sheet = Some(SheetParams { thickness: s("1.5"), radius: s("2"), k: s("0.44"), ..Default::default() });
    let mut data = SketchData::default();
    let mut al = ids(14_000_000);
    let o = data.add_point(&mut al, 0., 0.);
    let a = data.add_point(&mut al, 80., 0.);
    let b = data.add_point(&mut al, 0., 80.);
    data.add_curve(&mut al, Curve::Line { a: o, b: a }, false);
    data.add_curve(&mut al, Curve::Arc { c: o, s: a, e: b }, false);
    data.add_curve(&mut al, Curve::Line { a: b, b: o }, false);
    let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
    d.add_feature(FeatureKind::SmWall(SmWall { sketch: sk, reverse: false, offset: String::new(), mid: false, surface: None }));
    let edge: Vec<[f64; 3]> = (0..=32)
        .map(|i| {
            let t = i as f64 / 32. * std::f64::consts::FRAC_PI_2;
            [80. * t.cos(), 80. * t.sin(), 1.5]
        })
        .collect();
    d.add_feature(FeatureKind::SmCurveFlange(SmCurveFlange { edge, length: s("18"), flip: false, angle: s("90"), radius: None, side: 0 }));
    d.hide_consumed(d.features[1].id);
    d
}

fn quadro() -> Document {
    let mut d = Document::new("Quadro de mesa");
    let top = [[0., 0., 700.], [800., 0., 700.], [800., 500., 700.], [0., 500., 700.]];
    let mut paths = vec![];
    for i in 0..4 {
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed(top[i]), b: PointRef::Fixed(top[(i + 1) % 4]), start: s("0"), end: s("0") });
        paths.push(CurveRef::Ref(l));
    }
    let mut legs = vec![];
    for p in top {
        let l = d.add_geometry(RefKind::LinePoints { a: PointRef::Fixed([p[0], p[1], 0.]), b: PointRef::Fixed([p[0], p[1], 680.]), start: s("0"), end: s("0") });
        legs.push(CurveRef::Ref(l));
    }
    let mk = |paths: Vec<CurveRef>, corner| StructMember {
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
        corner,
        gap: s("0"),
        joints: vec![],
    };
    d.add_feature(FeatureKind::StructMember(mk(paths, CornerMode::MeiaEsquadria)));
    d.add_feature(FeatureKind::StructMember(mk(legs, CornerMode::Nenhum)));
    for r in &mut d.references {
        r.visible = false;
    }
    d
}

/// Todos os exemplos.
pub fn all() -> Vec<Example> {
    vec![
        Example { name: "Placa paramétrica", desc: "Furos, rasgo, rebaixos e filetes guiados por parâmetros", bench: Bench::Solidos, doc: fenkra_solidos::example_document() },
        Example { name: "Flange", desc: "Disco com furação em círculo e cubo central", bench: Bench::Solidos, doc: flange() },
        Example { name: "Caixa com casca", desc: "Perfil oblongo extrudado e esvaziado (Shell)", bench: Bench::Solidos, doc: caixa() },
        Example { name: "Suporte de chapa", desc: "Aba com alívio, bainha e veneziana — dobra e planifica", bench: Bench::Chapa, doc: suporte() },
        Example { name: "Bandeja curva", desc: "Aba em aresta curva com raio de dobra", bench: Bench::Chapa, doc: bandeja() },
        Example { name: "Quadro de mesa", desc: "Tubos quadrados com meia-esquadria (estrutural)", bench: Bench::Estrutural, doc: quadro() },
    ]
}

// ------------------------------------------------------------ prévias

/// Malha simplificada para a prévia (triângulos e arestas vivas), centrada na origem.
#[derive(Clone, Default)]
pub struct Preview {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub tris: Vec<[u32; 3]>,
    pub edges: Vec<[[f32; 3]; 2]>,
    pub color: [u8; 3],
    pub radius: f32,
}

/// Prévia de um documento recalculado.
pub fn preview_of(doc: &Document) -> Preview {
    let r = fenkra_solidos::recompute(doc);
    let mut p = Preview::default();
    for b in r.bodies.iter() {
        if p.tris.is_empty() {
            p.color = b.color;
        }
        let m = &b.mesh;
        let base = p.pos.len() as u32;
        for (q, n) in m.positions.chunks_exact(3).zip(m.normals.chunks_exact(3)) {
            p.pos.push([q[0], q[1], q[2]]);
            p.nrm.push([n[0], n[1], n[2]]);
        }
        for t in m.indices.chunks_exact(3) {
            p.tris.push([base + t[0], base + t[1], base + t[2]]);
        }
        let nseg = m.edges.len() / 6;
        for e in 0..m.edge_ranges.len() {
            if m.edge_smooth.get(e).is_some_and(|s| *s != 0) {
                continue;
            }
            let s0 = m.edge_ranges[e] as usize;
            let s1 = m.edge_ranges.get(e + 1).map(|x| *x as usize).unwrap_or(nseg);
            for q in m.edges[s0 * 6..s1 * 6].chunks_exact(6) {
                p.edges.push([[q[0], q[1], q[2]], [q[3], q[4], q[5]]]);
            }
        }
    }
    if p.pos.is_empty() {
        return p;
    }
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for q in &p.pos {
        for i in 0..3 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    let c = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.);
    for q in p.pos.iter_mut().chain(p.edges.iter_mut().flatten()) {
        for i in 0..3 {
            q[i] -= c[i];
        }
    }
    p.radius = p.pos.iter().map(|q| (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt()).fold(0., f32::max).max(1e-3);
    p
}

/// Desenha a prévia girando em torno de Z (vista isométrica, luz de cima), num retângulo.
/// Triângulos e arestas (faixas finas) vão ordenados pela profundidade numa só malha: as
/// arestas de trás ficam escondidas pelas faces, como na viewport.
pub fn paint_preview(p: &egui::Painter, rect: egui::Rect, pv: &Preview, angle: f32) {
    use egui::{pos2, Color32};
    if pv.tris.is_empty() {
        return;
    }
    let (sa, ca) = angle.sin_cos();
    let el = 0.72f32; // elevação da câmera
    let (se, ce) = el.sin_cos();
    let s = rect.width().min(rect.height()) * 0.46 / pv.radius;
    let c = rect.center();
    // vista: gira em Z, depois inclina; devolve (x tela, y tela, profundidade: maior = mais perto)
    let view = |q: [f32; 3]| {
        let x = q[0] * ca - q[1] * sa;
        let y = q[0] * sa + q[1] * ca;
        let z = q[2];
        let yv = y * ce - z * se; // para dentro da tela
        let zv = y * se + z * ce; // para cima
        (x, zv, -yv)
    };
    let rotn = |n: [f32; 3]| {
        let x = n[0] * ca - n[1] * sa;
        let y = n[0] * sa + n[1] * ca;
        [x, n[2] * ce + y * se, -(y * ce - n[2] * se)]
    };
    let vs: Vec<(f32, f32, f32)> = pv.pos.iter().map(|q| view(*q)).collect();
    let light = {
        let l = [-0.35f32, 0.75, 0.55];
        let k = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        l.map(|x| x / k)
    };
    let base = pv.color.map(|x| x as f32);
    let shade = |n: [f32; 3]| {
        let rn = rotn(n);
        let d = (rn[0] * light[0] + rn[1] * light[1] + rn[2] * light[2]).abs();
        let k = 0.42 + 0.58 * d;
        let spec = d.powf(24.) * 40.;
        Color32::from_rgb(
            (base[0] * k + spec).min(255.) as u8,
            (base[1] * k + spec).min(255.) as u8,
            (base[2] * k + spec).min(255.) as u8,
        )
    };
    let sc = |v: (f32, f32, f32)| pos2(c.x + v.0 * s, c.y - v.1 * s);
    // itens: (profundidade, tri ou aresta)
    let mut items: Vec<(f32, u32, bool)> = Vec::with_capacity(pv.tris.len() + pv.edges.len());
    for (i, t) in pv.tris.iter().enumerate() {
        let d = (vs[t[0] as usize].2 + vs[t[1] as usize].2 + vs[t[2] as usize].2) / 3.;
        items.push((d, i as u32, false));
    }
    let ev: Vec<[(f32, f32, f32); 2]> = pv.edges.iter().map(|e| [view(e[0]), view(e[1])]).collect();
    let bias = pv.radius * 0.01;
    for (i, e) in ev.iter().enumerate() {
        items.push(((e[0].2 + e[1].2) / 2. + bias, i as u32, true));
    }
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut m = egui::Mesh::default();
    let ink = Color32::from_rgb(18, 18, 26);
    for (_, i, is_edge) in items {
        if is_edge {
            let e = ev[i as usize];
            let (a, b) = (sc(e[0]), sc(e[1]));
            let d = b - a;
            let l = d.length();
            if l < 0.01 {
                continue;
            }
            let n = egui::vec2(-d.y, d.x) / l * 0.6;
            let k = m.vertices.len() as u32;
            for q in [a + n, b + n, b - n, a - n] {
                m.colored_vertex(q, ink);
            }
            m.add_triangle(k, k + 1, k + 2);
            m.add_triangle(k, k + 2, k + 3);
        } else {
            let t = pv.tris[i as usize];
            let k = m.vertices.len() as u32;
            for j in t {
                m.colored_vertex(sc(vs[j as usize]), shade(pv.nrm[j as usize]));
            }
            m.add_triangle(k, k + 1, k + 2);
        }
    }
    p.add(egui::Shape::mesh(m));
}

/// Prévias calculadas numa thread à parte (exemplos e peças recentes), por chave.
pub struct PreviewCache {
    map: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Option<std::sync::Arc<Preview>>>>>,
    tx: std::sync::mpsc::Sender<(String, Box<dyn FnOnce() -> Option<Document> + Send>)>,
}

impl Default for PreviewCache {
    fn default() -> Self {
        let map: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Option<std::sync::Arc<Preview>>>>> = Default::default();
        let (tx, rx) = std::sync::mpsc::channel::<(String, Box<dyn FnOnce() -> Option<Document> + Send>)>();
        let m = map.clone();
        std::thread::spawn(move || {
            for (k, f) in rx {
                let pv = f().map(|d| preview_of(&d)).filter(|p| !p.tris.is_empty()).map(std::sync::Arc::new);
                if let Ok(mut g) = m.lock() {
                    g.insert(k, pv);
                }
            }
        });
        Self { map, tx }
    }
}

impl PreviewCache {
    /// A prévia (se pronta); pede o cálculo na primeira vez.
    pub fn get(&self, key: &str, make: impl FnOnce() -> Option<Document> + Send + 'static) -> Option<std::sync::Arc<Preview>> {
        let mut g = self.map.lock().ok()?;
        match g.get(key) {
            Some(p) => p.clone(),
            None => {
                g.insert(key.to_string(), None);
                let _ = self.tx.send((key.to_string(), Box::new(make)));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exemplos_recalculam_sem_erro() {
        for e in super::all() {
            let r = fenkra_solidos::recompute(&e.doc);
            for f in &e.doc.features {
                if let Some(fenkra_solidos::Status::Error(m) | fenkra_solidos::Status::Warning(m)) = r.status.get(&f.id) {
                    panic!("{}: {} → {m}", e.name, f.name);
                }
            }
            assert!(!r.bodies.is_empty(), "{} sem corpo", e.name);
        }
    }
}
