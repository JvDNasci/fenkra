//! Kernel geométrico do SolidCadCraft (ADR 0001): OpenCASCADE atrás de uma API Rust segura.
//! O resto do sistema só conhece este módulo, nunca o OCCT diretamente.

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct KernelError(pub String);

pub type KResult<T> = Result<T, KernelError>;

/// Plano de trabalho: origem, direção X e normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: [f64; 3],
    pub x: [f64; 3],
    pub normal: [f64; 3],
}

impl Frame {
    pub fn y(&self) -> [f64; 3] {
        cross(self.normal, self.x)
    }
    /// Ponto 3D a partir de coordenadas (u, v) do plano.
    pub fn point(&self, u: f64, v: f64) -> [f64; 3] {
        let y = self.y();
        [0, 1, 2].map(|i| self.origin[i] + u * self.x[i] + v * y[i])
    }
    pub fn offset(&self, d: f64) -> Frame {
        Frame { origin: [0, 1, 2].map(|i| self.origin[i] + d * self.normal[i]), ..*self }
    }
}

pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Transformação rígida ou escala de um sólido.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Xform {
    Translate([f64; 3]),
    /// origem, direção do eixo, graus
    Rotate([f64; 3], [f64; 3], f64),
    /// ponto e normal do plano de simetria
    Mirror([f64; 3], [f64; 3]),
    /// centro e fator
    Scale([f64; 3], f64),
}

impl Xform {
    fn params(&self) -> (u8, Vec<f64>) {
        match *self {
            Xform::Translate(d) => (0, d.to_vec()),
            Xform::Rotate(o, d, a) => (1, vec![o[0], o[1], o[2], d[0], d[1], d[2], a]),
            Xform::Mirror(o, n) => (2, vec![o[0], o[1], o[2], n[0], n[1], n[2]]),
            Xform::Scale(o, k) => (3, vec![o[0], o[1], o[2], k]),
        }
    }
}

/// Medidas de uma face: área, tipo (0 plano, 1 cilindro, 2 cone, 3 esfera, 4 toro, 5 livre),
/// direção (normal do plano / eixo), ponto de referência e raio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceProps {
    pub area: f64,
    pub kind: u8,
    pub dir: [f64; 3],
    pub origin: [f64; 3],
    pub radius: f64,
}

/// Face cilíndrica: eixo, raio, extensão ao longo do eixo e se é interna (furo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    pub origin: [f64; 3],
    pub dir: [f64; 3],
    pub radius: f64,
    pub h0: f64,
    pub h1: f64,
    pub internal: bool,
}

/// Segmento de perfil 2D (coordenadas do plano do sketch).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg2 {
    Line([f64; 2], [f64; 2]),
    /// Arco por três pontos: início, meio, fim.
    Arc([f64; 2], [f64; 2], [f64; 2]),
    Circle([f64; 2], f64),
}

/// Laço fechado de segmentos.
pub type Loop2 = Vec<Seg2>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bool {
    Fuse = 0,
    Cut = 1,
    Common = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HoleShape {
    Simple,
    Counterbore { diameter: f64, depth: f64 },
    Countersink { diameter: f64, angle: f64 },
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub edges: Vec<f32>,
    /// início (em `indices`) dos triângulos de cada face
    pub face_ranges: Vec<u32>,
    /// início (em segmentos de `edges`) de cada aresta
    pub edge_ranges: Vec<u32>,
    /// ponto médio (xyz) de cada aresta
    pub edge_mids: Vec<f32>,
    /// posição (xyz) de cada vértice
    pub vertices: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MassProps {
    pub volume: f64,
    pub area: f64,
    pub center: [f64; 3],
}

#[cfg(occt)]
mod ffi;

#[cfg(occt)]
mod imp {
    use super::*;
    use cxx::UniquePtr;
    use ffi::bridge as b;

    pub const AVAILABLE: bool = true;

    pub struct Shape(UniquePtr<b::Shape>);

    impl Clone for Shape {
        fn clone(&self) -> Self {
            Shape(b::shape_clone(&self.0))
        }
    }

    fn err(e: cxx::Exception) -> KernelError {
        KernelError(e.what().to_string())
    }

    fn frame(f: &Frame) -> b::Frame {
        b::Frame {
            ox: f.origin[0],
            oy: f.origin[1],
            oz: f.origin[2],
            xx: f.x[0],
            xy: f.x[1],
            xz: f.x[2],
            nx: f.normal[0],
            ny: f.normal[1],
            nz: f.normal[2],
        }
    }

    impl Shape {
        pub fn empty() -> Self {
            Shape(b::shape_empty())
        }
        pub fn is_null(&self) -> bool {
            b::shape_is_null(&self.0)
        }

        pub fn face(f: &Frame, loops: &[Loop2]) -> KResult<Shape> {
            let mut segs = vec![];
            let mut ends = vec![];
            for l in loops {
                for s in l {
                    let z = b::Seg { kind: 0, x1: 0., y1: 0., x2: 0., y2: 0., x3: 0., y3: 0. };
                    segs.push(match *s {
                        Seg2::Line(a, c) => b::Seg { kind: 0, x1: a[0], y1: a[1], x2: c[0], y2: c[1], ..z },
                        Seg2::Arc(a, m, c) => b::Seg { kind: 1, x1: a[0], y1: a[1], x2: m[0], y2: m[1], x3: c[0], y3: c[1] },
                        Seg2::Circle(c, r) => b::Seg { kind: 2, x1: c[0], y1: c[1], x2: r, ..z },
                    });
                }
                ends.push(segs.len() as u32);
            }
            b::make_face(&frame(f), &segs, &ends).map(Shape).map_err(err)
        }

        pub fn extrude(&self, dir: [f64; 3], front: f64, back: f64) -> KResult<Shape> {
            b::extrude(&self.0, dir[0], dir[1], dir[2], front, back).map(Shape).map_err(err)
        }

        pub fn revolve(&self, origin: [f64; 3], dir: [f64; 3], angle_deg: f64, start_deg: f64) -> KResult<Shape> {
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            b::revolve(&self.0, ox, oy, oz, dx, dy, dz, angle_deg, start_deg).map(Shape).map_err(err)
        }

        pub fn boolean(&self, other: &Shape, op: Bool) -> KResult<Shape> {
            b::boolean(&self.0, &other.0, op as u8).map(Shape).map_err(err)
        }

        pub fn hole_tool(origin: [f64; 3], dir: [f64; 3], diameter: f64, depth: f64, shape: HoleShape) -> KResult<Shape> {
            let (k, d2, p2) = match shape {
                HoleShape::Simple => (0, 0., 0.),
                HoleShape::Counterbore { diameter, depth } => (1, diameter, depth),
                HoleShape::Countersink { diameter, angle } => (2, diameter, angle),
            };
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            b::hole_tool(ox, oy, oz, dx, dy, dz, diameter, depth, k, d2, p2).map(Shape).map_err(err)
        }

        pub fn fillet_all(&self, r: f64) -> KResult<Shape> {
            b::fillet_all(&self.0, r).map(Shape).map_err(err)
        }
        pub fn chamfer_all(&self, d: f64) -> KResult<Shape> {
            b::chamfer_all(&self.0, d).map(Shape).map_err(err)
        }
        pub fn compound(&self, other: &Shape) -> Shape {
            Shape(b::compound(&self.0, &other.0))
        }

        /// Booleana que propaga etiquetas de face: devolve (resultado, etiqueta de cada face; -1 = nova).
        pub fn boolean_tagged(&self, other: &Shape, op: Bool, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let mut out = vec![];
            let s = b::boolean_t(&self.0, &other.0, op as u8, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        /// Filete (`chamfer = false`) ou chanfro nas arestas mais próximas dos pontos médios dados.
        pub fn edges_tagged(&self, v: f64, mids: &[[f64; 3]], tol: f64, chamfer: bool, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = mids.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = if chamfer { b::chamfer_sel_t(&self.0, v, &flat, tol, tags, &mut out) } else { b::fillet_sel_t(&self.0, v, &flat, tol, tags, &mut out) }
                .map_err(err)?;
            Ok((Shape(s), out))
        }
        /// Ferramenta de rosca modelada (subtrair do corpo).
        pub fn thread_tool(origin: [f64; 3], dir: [f64; 3], major_d: f64, pitch: f64, length: f64, angle_deg: f64) -> KResult<Shape> {
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            b::thread_tool(ox, oy, oz, dx, dy, dz, major_d, pitch, length, angle_deg).map(Shape).map_err(err)
        }
        /// Extrusão até o plano (origem, normal).
        pub fn extrude_until(&self, dir: [f64; 3], len: f64, origin: [f64; 3], normal: [f64; 3]) -> KResult<Shape> {
            let [dx, dy, dz] = dir;
            let [ox, oy, oz] = origin;
            let [nx, ny, nz] = normal;
            b::extrude_until(&self.0, dx, dy, dz, len, ox, oy, oz, nx, ny, nz).map(Shape).map_err(err)
        }
        pub fn fillet_tagged(&self, r: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let mut out = vec![];
            let s = b::fillet_all_t(&self.0, r, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn chamfer_tagged(&self, d: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let mut out = vec![];
            let s = b::chamfer_all_t(&self.0, d, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }

        /// Transformação: translação, rotação, espelho ou escala (ver `Xform`).
        pub fn transformed(&self, x: Xform) -> KResult<Shape> {
            let (k, p) = x.params();
            b::transform_shape(&self.0, k, &p).map(Shape).map_err(err)
        }
        pub fn transformed_tagged(&self, x: Xform, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let (k, p) = x.params();
            let mut out = vec![];
            let s = b::transform_t(&self.0, k, &p, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn sweep(&self, spine: &[[f64; 3]], mode: u8) -> KResult<Shape> {
            let flat: Vec<f64> = spine.iter().flatten().copied().collect();
            b::sweep_tool(&self.0, &flat, mode).map(Shape).map_err(err)
        }
        /// Multi-seções pelas faces deste composto, na ordem.
        pub fn loft(&self, ruled: bool, closed: bool) -> KResult<Shape> {
            b::loft_tool(&self.0, ruled, closed).map(Shape).map_err(err)
        }
        pub fn primitive(kind: u8, f: &Frame, p: &[f64]) -> KResult<Shape> {
            b::primitive(kind, &frame(f), p).map(Shape).map_err(err)
        }
        pub fn shell_tagged(&self, faces: &[[f64; 3]], t: f64, outward: bool, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = b::shell_t(&self.0, &flat, t, outward, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn draft_tagged(&self, faces: &[[f64; 3]], pull: [f64; 3], angle: f64, neutral: &Frame, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            let (o, n) = (neutral.origin, neutral.normal);
            let mut out = vec![];
            let s = b::draft_t(&self.0, &flat, pull[0], pull[1], pull[2], angle, o[0], o[1], o[2], n[0], n[1], n[2], tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn faces_offset_tool(&self, faces: &[[f64; 3]], value: f64) -> KResult<Shape> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            b::faces_offset_tool(&self.0, &flat, value).map(Shape).map_err(err)
        }
        pub fn extrude_taper(&self, dir: [f64; 3], len: f64, angle_deg: f64) -> KResult<Shape> {
            b::extrude_taper(&self.0, dir[0], dir[1], dir[2], len, angle_deg).map(Shape).map_err(err)
        }
        pub fn thin(&self, t: f64) -> KResult<Shape> {
            b::thin_profile(&self.0, t).map(Shape).map_err(err)
        }
        pub fn thick_segments(segs: &[([f64; 3], [f64; 3])], normal: [f64; 3], t: f64) -> KResult<Shape> {
            let flat: Vec<f64> = segs.iter().flat_map(|(a, c)| a.iter().chain(c.iter()).copied().collect::<Vec<_>>()).collect();
            b::thick_segments(&flat, normal[0], normal[1], normal[2], t).map(Shape).map_err(err)
        }
        /// modo 0 filete variável, 1 chanfro 2 distâncias, 2 chanfro distância + ângulo
        pub fn edges_mode_tagged(&self, mode: u8, v1: f64, v2: f64, mids: &[[f64; 3]], tol: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = mids.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = b::edges_mode_t(&self.0, mode, v1, v2, &flat, tol, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn cylinder_at(&self, p: [f64; 3]) -> KResult<Cylinder> {
            let v = b::cylinder_at(&self.0, p[0], p[1], p[2]).map_err(err)?;
            Ok(Cylinder { origin: [v[0], v[1], v[2]], dir: [v[3], v[4], v[5]], radius: v[6], h0: v[7], h1: v[8], internal: v[9] > 0.5 })
        }
        pub fn thread_tool_ext(origin: [f64; 3], dir: [f64; 3], major_d: f64, pitch: f64, length: f64, angle_deg: f64) -> KResult<Shape> {
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            b::thread_tool_ext(ox, oy, oz, dx, dy, dz, major_d, pitch, length, angle_deg).map(Shape).map_err(err)
        }

        pub fn face_copy(&self, p: [f64; 3]) -> KResult<Shape> {
            b::face_copy(&self.0, p[0], p[1], p[2]).map(Shape).map_err(err)
        }
        pub fn offset_surface(&self, d: f64) -> KResult<Shape> {
            b::offset_surface(&self.0, d).map(Shape).map_err(err)
        }
        pub fn fill_surface(curves: &[Vec<[f64; 3]>]) -> KResult<Shape> {
            let mut flat = vec![];
            let mut ends = vec![];
            for c in curves {
                flat.extend(c.iter().flatten().copied());
                ends.push((flat.len() / 3) as u32);
            }
            b::fill_surface(&flat, &ends).map(Shape).map_err(err)
        }
        pub fn extrude_curve(pts: &[[f64; 3]], dir: [f64; 3], len: f64) -> KResult<Shape> {
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            b::extrude_curve(&flat, dir[0], dir[1], dir[2], len).map(Shape).map_err(err)
        }
        pub fn revolve_curve(pts: &[[f64; 3]], origin: [f64; 3], dir: [f64; 3], angle_deg: f64) -> KResult<Shape> {
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            b::revolve_curve(&flat, origin[0], origin[1], origin[2], dir[0], dir[1], dir[2], angle_deg).map(Shape).map_err(err)
        }
        /// Ponto mais próximo na forma e a normal da face nesse ponto.
        pub fn surface_near(&self, p: [f64; 3]) -> KResult<([f64; 3], [f64; 3])> {
            let v = b::surface_near(&self.0, p[0], p[1], p[2]).map_err(err)?;
            Ok(([v[0], v[1], v[2]], [v[3], v[4], v[5]]))
        }
        pub fn face_props(&self, p: [f64; 3]) -> KResult<FaceProps> {
            let v = b::face_props(&self.0, p[0], p[1], p[2]).map_err(err)?;
            Ok(FaceProps { area: v[0], kind: v[1] as u8, dir: [v[2], v[3], v[4]], origin: [v[5], v[6], v[7]], radius: v[8] })
        }
        pub fn extrude_to_next(&self, dir: [f64; 3], len: f64, body: &Shape) -> KResult<Shape> {
            b::extrude_to_next(&self.0, dir[0], dir[1], dir[2], len, &body.0).map(Shape).map_err(err)
        }
        /// Divide a ferramenta pela superfície e fica com o pedaço que passa por `start`.
        pub fn clip_until(&self, cutter: &Shape, start: [f64; 3], extend: bool) -> KResult<Shape> {
            b::clip_until(&self.0, &cutter.0, start[0], start[1], start[2], extend).map(Shape).map_err(err)
        }
        pub fn plane_face(o: [f64; 3], n: [f64; 3], half: f64) -> KResult<Shape> {
            b::plane_face(o[0], o[1], o[2], n[0], n[1], n[2], half).map(Shape).map_err(err)
        }
        pub fn same_face(&self, a: [f64; 3], c: [f64; 3]) -> bool {
            b::same_face(&self.0, a[0], a[1], a[2], c[0], c[1], c[2]).unwrap_or(false)
        }
        pub fn full_round_tagged(&self, mid: [f64; 3], width: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let mut out = vec![];
            let s = b::full_round_t(&self.0, mid[0], mid[1], mid[2], width, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn face_face_fillet_tagged(&self, a: [f64; 3], c: [f64; 3], r: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let mut out = vec![];
            let s = b::face_face_fillet_t(&self.0, a[0], a[1], a[2], c[0], c[1], c[2], r, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }

        pub fn mesh(&self, deflection: f64) -> KResult<Mesh> {
            let m = b::tessellate(&self.0, deflection).map_err(err)?;
            Ok(Mesh {
                positions: m.positions,
                normals: m.normals,
                indices: m.indices,
                edges: m.edges,
                face_ranges: m.face_ranges,
                edge_ranges: m.edge_ranges,
                edge_mids: m.edge_mids,
                vertices: m.vertices,
            })
        }
        pub fn bbox(&self) -> Option<([f64; 3], [f64; 3])> {
            let v = b::bounding_box(&self.0);
            (v.len() == 6).then(|| ([v[0], v[1], v[2]], [v[3], v[4], v[5]]))
        }
        pub fn mass(&self) -> MassProps {
            let v = b::mass_props(&self.0);
            if v.len() < 5 {
                return MassProps::default();
            }
            MassProps { volume: v[0], area: v[1], center: [v[2], v[3], v[4]] }
        }
        pub fn face_count(&self) -> u32 {
            b::count_faces(&self.0)
        }
        pub fn export_step(&self, path: &str) -> KResult<()> {
            b::export_step(&self.0, path).map_err(err)
        }
        pub fn export_stl(&self, path: &str) -> KResult<()> {
            b::export_stl(&self.0, path).map_err(err)
        }
        pub fn import_step(path: &str) -> KResult<Shape> {
            b::import_step(path).map(Shape).map_err(err)
        }
    }

    pub fn version() -> String {
        format!("OpenCASCADE {}", b::occt_version())
    }
}

#[cfg(not(occt))]
mod imp {
    //! Modo sem kernel: compila e roda a interface, mas toda operação geométrica falha.
    use super::*;

    pub const AVAILABLE: bool = false;

    #[derive(Clone)]
    pub struct Shape;

    fn no<T>() -> KResult<T> {
        Err(KernelError("kernel geométrico indisponível (OpenCASCADE não encontrado na compilação)".into()))
    }

    impl Shape {
        pub fn empty() -> Self {
            Shape
        }
        pub fn is_null(&self) -> bool {
            true
        }
        pub fn face(_: &Frame, _: &[Loop2]) -> KResult<Shape> {
            no()
        }
        pub fn extrude(&self, _: [f64; 3], _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn revolve(&self, _: [f64; 3], _: [f64; 3], _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn boolean(&self, _: &Shape, _: Bool) -> KResult<Shape> {
            no()
        }
        pub fn hole_tool(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: HoleShape) -> KResult<Shape> {
            no()
        }
        pub fn fillet_all(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn chamfer_all(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn compound(&self, _: &Shape) -> Shape {
            Shape
        }
        pub fn boolean_tagged(&self, _: &Shape, _: Bool, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn edges_tagged(&self, _: f64, _: &[[f64; 3]], _: f64, _: bool, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn extrude_until(&self, _: [f64; 3], _: f64, _: [f64; 3], _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn thread_tool(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn fillet_tagged(&self, _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn chamfer_tagged(&self, _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn transformed(&self, _: Xform) -> KResult<Shape> {
            no()
        }
        pub fn transformed_tagged(&self, _: Xform, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn sweep(&self, _: &[[f64; 3]], _: u8) -> KResult<Shape> {
            no()
        }
        pub fn loft(&self, _: bool, _: bool) -> KResult<Shape> {
            no()
        }
        pub fn primitive(_: u8, _: &Frame, _: &[f64]) -> KResult<Shape> {
            no()
        }
        pub fn shell_tagged(&self, _: &[[f64; 3]], _: f64, _: bool, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn draft_tagged(&self, _: &[[f64; 3]], _: [f64; 3], _: f64, _: &Frame, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn faces_offset_tool(&self, _: &[[f64; 3]], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn extrude_taper(&self, _: [f64; 3], _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn thin(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn thick_segments(_: &[([f64; 3], [f64; 3])], _: [f64; 3], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn edges_mode_tagged(&self, _: u8, _: f64, _: f64, _: &[[f64; 3]], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn cylinder_at(&self, _: [f64; 3]) -> KResult<Cylinder> {
            no()
        }
        pub fn thread_tool_ext(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn face_copy(&self, _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn offset_surface(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn fill_surface(_: &[Vec<[f64; 3]>]) -> KResult<Shape> {
            no()
        }
        pub fn extrude_curve(_: &[[f64; 3]], _: [f64; 3], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn revolve_curve(_: &[[f64; 3]], _: [f64; 3], _: [f64; 3], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn surface_near(&self, _: [f64; 3]) -> KResult<([f64; 3], [f64; 3])> {
            no()
        }
        pub fn face_props(&self, _: [f64; 3]) -> KResult<FaceProps> {
            no()
        }
        pub fn clip_until(&self, _: &Shape, _: [f64; 3], _: bool) -> KResult<Shape> {
            no()
        }
        pub fn plane_face(_: [f64; 3], _: [f64; 3], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn extrude_to_next(&self, _: [f64; 3], _: f64, _: &Shape) -> KResult<Shape> {
            no()
        }
        pub fn same_face(&self, _: [f64; 3], _: [f64; 3]) -> bool {
            false
        }
        pub fn full_round_tagged(&self, _: [f64; 3], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn face_face_fillet_tagged(&self, _: [f64; 3], _: [f64; 3], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn mesh(&self, _: f64) -> KResult<Mesh> {
            no()
        }
        pub fn bbox(&self) -> Option<([f64; 3], [f64; 3])> {
            None
        }
        pub fn mass(&self) -> MassProps {
            MassProps::default()
        }
        pub fn face_count(&self) -> u32 {
            0
        }
        pub fn export_step(&self, _: &str) -> KResult<()> {
            no()
        }
        pub fn export_stl(&self, _: &str) -> KResult<()> {
            no()
        }
        pub fn import_step(_: &str) -> KResult<Shape> {
            no()
        }
    }

    pub fn version() -> String {
        "sem kernel".into()
    }
}

pub use imp::{version, Shape, AVAILABLE};

#[cfg(all(test, occt))]
mod tests {
    use super::*;

    const XY: Frame = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };

    fn rect(w: f64, h: f64) -> Loop2 {
        let (a, b, c, d) = ([-w / 2., -h / 2.], [w / 2., -h / 2.], [w / 2., h / 2.], [-w / 2., h / 2.]);
        vec![Seg2::Line(a, b), Seg2::Line(b, c), Seg2::Line(c, d), Seg2::Line(d, a)]
    }

    #[test]
    fn extrusao_com_furo_no_perfil() {
        let f = Shape::face(&XY, &[rect(100., 50.), vec![Seg2::Circle([0., 0.], 10.)]]).unwrap();
        let s = f.extrude([0., 0., 1.], 20., 0.).unwrap();
        let v = s.mass().volume;
        let esperado = (100. * 50. - std::f64::consts::PI * 100.) * 20.;
        assert!((v - esperado).abs() < 1e-3, "{v} vs {esperado}");
        let m = s.mesh(0.1).unwrap();
        assert!(!m.indices.is_empty() && !m.edges.is_empty());
    }

    #[test]
    fn furo_rebaixado_e_filete() {
        let caixa = Shape::face(&XY, &[rect(40., 40.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let t = Shape::hole_tool([0., 0., 10.], [0., 0., -1.], 6.6, 10., HoleShape::Counterbore { diameter: 11., depth: 6.4 }).unwrap();
        let furada = caixa.boolean(&t, Bool::Cut).unwrap();
        assert!(furada.mass().volume < caixa.mass().volume);
        assert!(caixa.fillet_all(1.).is_ok());
    }

    #[test]
    fn etiquetas_de_face_sobrevivem_a_booleana() {
        let caixa = Shape::face(&XY, &[rect(40., 40.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let n = caixa.face_count() as i32;
        assert_eq!(n, 6);
        let tags = vec![7; n as usize];
        let t = Shape::hole_tool([0., 0., 10.], [0., 0., -1.], 6., 10., HoleShape::Simple).unwrap();
        let (furada, out) = caixa.boolean_tagged(&t, Bool::Cut, &tags).unwrap();
        assert_eq!(out.len() as u32, furada.face_count());
        // as 6 faces da caixa continuam com a etiqueta 7; a parede do furo é nova (-1)
        assert_eq!(out.iter().filter(|t| **t == 7).count(), 6);
        assert!(out.contains(&-1));
        assert_eq!(furada.mesh(0.1).unwrap().face_ranges.len(), out.len());
        let (_, fo) = furada.fillet_tagged(0.5, &out).unwrap();
        assert!(fo.contains(&7) && fo.contains(&-1));
    }

    #[test]
    fn extrusao_ate_plano_inclinado_e_filete_em_aresta() {
        let f = Shape::face(&XY, &[rect(20., 20.)]).unwrap();
        // plano a 45° passando por z = 20 no centro
        let n = [0., -std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2];
        let s = f.extrude_until([0., 0., 1.], 200., [0., 0., 20.], n).unwrap();
        let v = s.mass().volume;
        assert!((v - 20. * 20. * 20.).abs() < 1e-3, "{v}");
        let caixa = f.extrude([0., 0., 1.], 10., 0.).unwrap();
        let m = caixa.mesh(0.1).unwrap();
        assert_eq!(m.edge_mids.len() / 3, m.edge_ranges.len());
        assert_eq!(m.vertices.len() / 3, 8);
        let tags = vec![1; caixa.face_count() as usize];
        let aresta = [10., 0., 10.]; // aresta superior em x = 10
        let (r, _) = caixa.edges_tagged(2., &[aresta], 0.5, false, &tags).unwrap();
        assert!(r.face_count() == 7);
    }

    #[test]
    fn rosca_modelada_remove_material() {
        let bloco = Shape::face(&XY, &[rect(20., 20.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let furo = Shape::hole_tool([0., 0., 10.], [0., 0., -1.], 5.0, 10., HoleShape::Simple).unwrap();
        let furado = bloco.boolean(&furo, Bool::Cut).unwrap();
        let rosca = Shape::thread_tool([0., 0., 10.], [0., 0., -1.], 6.0, 1.0, 10., 60.).unwrap();
        let roscado = furado.boolean(&rosca, Bool::Cut).unwrap();
        let (v0, v1) = (furado.mass().volume, roscado.mass().volume);
        assert!(v1 < v0 && v1 > v0 * 0.95, "{v0} -> {v1}");
    }

    #[test]
    fn revolucao_cilindro() {
        let l = rect(10., 20.).into_iter().map(|s| match s {
            Seg2::Line(a, b) => Seg2::Line([a[0] + 5., a[1]], [b[0] + 5., b[1]]),
            o => o,
        });
        let f = Shape::face(&XY, &[l.collect()]).unwrap();
        let s = f.revolve([0.; 3], [0., 1., 0.], 360., 0.).unwrap();
        let esperado = std::f64::consts::PI * 100. * 20.;
        assert!((s.mass().volume - esperado).abs() < 1e-2);
    }
}
