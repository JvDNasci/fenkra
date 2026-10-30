//! Kernel geométrico do Fenkra (ADR 0001): OpenCASCADE atrás de uma API Rust segura.
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

/// Parâmetros de uma aba de chapa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetFlange {
    pub t: f64,
    pub r: f64,
    pub angle: f64,
    pub length: f64,
    pub off1: f64,
    pub off2: f64,
    pub flip: bool,
    pub flat: bool,
    pub k: f64,
}

/// Medidas da aresta da aba.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetEdgeInfo {
    pub length: f64,
    pub thickness: f64,
    pub normal: [f64; 3],
    pub out: [f64; 3],
    /// pontas da dobra (depois dos recuos), na face de referência
    pub ends: [[f64; 3]; 2],
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
#[derive(Clone, Debug, PartialEq)]
pub enum Seg2 {
    Line([f64; 2], [f64; 2]),
    /// Arco por três pontos: início, meio, fim.
    Arc([f64; 2], [f64; 2], [f64; 2]),
    Circle([f64; 2], f64),
    /// Spline que passa pelos pontos (B-spline interpolada).
    Spline(Vec<[f64; 2]>),
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
    /// rebaixo com fundo cônico (ângulo da broca do rebaixo)
    Counterdrill { diameter: f64, depth: f64, angle: f64 },
}

impl HoleShape {
    /// Profundidade da cabeça (rebaixo/escareado) a partir da boca do furo.
    pub fn head_depth(&self, hole_d: f64) -> f64 {
        match *self {
            HoleShape::Simple => 0.,
            HoleShape::Counterbore { depth, .. } => depth,
            HoleShape::Counterdrill { diameter, depth, angle } => depth + ((diameter - hole_d) / 2.).max(0.) / (angle.to_radians() / 2.).tan().max(1e-6),
            HoleShape::Countersink { diameter, angle } => ((diameter - hole_d) / 2.).max(0.) / (angle.to_radians() / 2.).tan().max(1e-6),
        }
    }
}

/// Sem face vizinha (em `Mesh::edge_faces` e `Topology::edge_faces`).
pub const NO_FACE: u32 = u32::MAX;

/// Assinatura geométrica de uma face para as referências v2: tipo (0 plano, 1 cilindro, 2 cone,
/// 3 esfera, 4 toro, 5 livre), normal externa (plano) ou eixo (cilindro/cone/toro, sentido
/// normalizado), área e um ponto garantidamente sobre a face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceInfo {
    pub kind: u8,
    pub dir: [f64; 3],
    pub area: f64,
    pub point: [f64; 3],
}

/// Topologia de um corpo sem tesselação: mesma ordem de faces e de arestas da malha.
#[derive(Clone, Debug, Default)]
pub struct Topology {
    pub faces: Vec<FaceInfo>,
    /// ponto médio de cada aresta (zeros nas costuras e arestas degeneradas)
    pub edge_mids: Vec<[f64; 3]>,
    /// as duas faces vizinhas de cada aresta (`NO_FACE` = nenhuma)
    pub edge_faces: Vec<[u32; 2]>,
}

fn face_infos(flat: &[f64]) -> Vec<FaceInfo> {
    flat.chunks_exact(8).map(|v| FaceInfo { kind: v[0] as u8, dir: [v[1], v[2], v[3]], area: v[4], point: [v[5], v[6], v[7]] }).collect()
}

fn pairs(flat: &[u32]) -> Vec<[u32; 2]> {
    flat.chunks_exact(2).map(|v| [v[0], v[1]]).collect()
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
    /// 1 por aresta suave (tangente entre as faces vizinhas), 0 por aresta viva
    pub edge_smooth: Vec<u8>,
    /// posição (xyz) de cada vértice
    pub vertices: Vec<f32>,
    /// as duas faces vizinhas de cada aresta (índices na ordem de `face_ranges`; `NO_FACE` = nenhuma)
    pub edge_faces: Vec<[u32; 2]>,
    /// assinatura de cada face, na ordem de `face_ranges`
    pub face_info: Vec<FaceInfo>,
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
                    match s {
                        Seg2::Line(a, c) => segs.push(b::Seg { kind: 0, x1: a[0], y1: a[1], x2: c[0], y2: c[1], ..z }),
                        Seg2::Arc(a, m, c) => segs.push(b::Seg { kind: 1, x1: a[0], y1: a[1], x2: m[0], y2: m[1], x3: c[0], y3: c[1] }),
                        Seg2::Circle(c, r) => segs.push(b::Seg { kind: 2, x1: c[0], y1: c[1], x2: *r, ..z }),
                        // spline: um registro por ponto (tipo 3); x2 = 1 marca o último ponto
                        Seg2::Spline(pts) => {
                            for (i, p) in pts.iter().enumerate() {
                                segs.push(b::Seg { kind: 3, x1: p[0], y1: p[1], x2: if i + 1 == pts.len() { 1. } else { 0. }, ..z });
                            }
                        }
                    }
                }
                ends.push(segs.len() as u32);
            }
            b::make_face(&frame(f), &segs, &ends).map(Shape).map_err(err)
        }

        pub fn extrude(&self, dir: [f64; 3], front: f64, back: f64) -> KResult<Shape> {
            self.extrude_roles(dir, front, back).map(|r| r.0)
        }
        /// Extrusão com o papel de cada face do resultado (na ordem de `face_ranges`):
        /// 0 = tampa inicial, 1 = tampa final, 2+i = lateral gerada pela aresta i do perfil, -1 = desconhecido.
        pub fn extrude_roles(&self, dir: [f64; 3], front: f64, back: f64) -> KResult<(Shape, Vec<i32>)> {
            let mut roles = vec![];
            let s = b::extrude(&self.0, dir[0], dir[1], dir[2], front, back, &mut roles).map_err(err)?;
            Ok((Shape(s), roles))
        }

        pub fn revolve(&self, origin: [f64; 3], dir: [f64; 3], angle_deg: f64, start_deg: f64) -> KResult<Shape> {
            self.revolve_roles(origin, dir, angle_deg, start_deg).map(|r| r.0)
        }
        /// Revolução com o papel de cada face (mesma convenção de `extrude_roles`).
        pub fn revolve_roles(&self, origin: [f64; 3], dir: [f64; 3], angle_deg: f64, start_deg: f64) -> KResult<(Shape, Vec<i32>)> {
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            let mut roles = vec![];
            let s = b::revolve(&self.0, ox, oy, oz, dx, dy, dz, angle_deg, start_deg, &mut roles).map_err(err)?;
            Ok((Shape(s), roles))
        }

        pub fn boolean(&self, other: &Shape, op: Bool) -> KResult<Shape> {
            b::boolean(&self.0, &other.0, op as u8).map(Shape).map_err(err)
        }

        pub fn hole_tool(origin: [f64; 3], dir: [f64; 3], diameter: f64, depth: f64, shape: HoleShape) -> KResult<Shape> {
            let (k, d2, p2) = match shape {
                HoleShape::Simple => (0, 0., 0.),
                HoleShape::Counterbore { diameter, depth } => (1, diameter, depth),
                HoleShape::Countersink { diameter, angle } => (2, diameter, angle),
                HoleShape::Counterdrill { .. } => return Self::hole_tool_ext(origin, dir, diameter, depth, shape, 0., 0.),
            };
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            b::hole_tool(ox, oy, oz, dx, dy, dz, diameter, depth, k, d2, p2).map(Shape).map_err(err)
        }
        /// Furo com fundo em V (`tip` = ângulo da ponta; 0 = plano) e corpo cônico (`taper`, meio-ângulo).
        pub fn hole_tool_ext(origin: [f64; 3], dir: [f64; 3], diameter: f64, depth: f64, shape: HoleShape, tip: f64, taper: f64) -> KResult<Shape> {
            let (k, d2, p2, a3) = match shape {
                HoleShape::Simple => (0, 0., 0., 0.),
                HoleShape::Counterbore { diameter, depth } => (1, diameter, depth, 0.),
                HoleShape::Countersink { diameter, angle } => (2, diameter, angle, 0.),
                HoleShape::Counterdrill { diameter, depth, angle } => (3, diameter, depth, angle),
            };
            b::hole_tool_ext(origin[0], origin[1], origin[2], dir[0], dir[1], dir[2], diameter, depth, k, d2, p2, a3, tip, taper).map(Shape).map_err(err)
        }
        /// Primeiro ponto onde a reta (p, d) cruza a forma: (ponto, parâmetro t).
        pub fn ray_hit(&self, p: [f64; 3], d: [f64; 3]) -> KResult<Option<([f64; 3], f64)>> {
            let v = b::ray_hit(&self.0, p[0], p[1], p[2], d[0], d[1], d[2]).map_err(err)?;
            Ok((v.len() == 4).then(|| ([v[0], v[1], v[2]], v[3])))
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
            self.boolean_tagged_src(other, op, tags).map(|(s, out, _)| (s, out))
        }
        /// Como `boolean_tagged`, mais a face da ferramenta (`other`) de onde vem cada face do
        /// resultado (-1 = nenhuma): os papéis das faces da ferramenta seguem por aí.
        pub fn boolean_tagged_src(&self, other: &Shape, op: Bool, tags: &[i32]) -> KResult<(Shape, Vec<i32>, Vec<i32>)> {
            let (mut out, mut out_b) = (vec![], vec![]);
            let s = b::boolean_t(&self.0, &other.0, op as u8, tags, &mut out, &mut out_b).map_err(err)?;
            Ok((Shape(s), out, out_b))
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
        /// Deslocada com a ligação escolhida (0 automático, 1 arredondado, 2 cantos vivos).
        pub fn offset_surface_join(&self, d: f64, join: u8) -> KResult<Shape> {
            b::offset_surface_j(&self.0, d, join).map(Shape).map_err(err)
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
        // ---------------- bancada de superfícies
        pub fn sphere_surface(c: [f64; 3], r: f64) -> KResult<Shape> {
            b::sphere_surface(c[0], c[1], c[2], r).map(Shape).map_err(err)
        }
        /// Face cilíndrica (sem tampas) do eixo `o`+`d`, entre as alturas h0 e h1.
        pub fn cylinder_surface(o: [f64; 3], d: [f64; 3], r: f64, h0: f64, h1: f64) -> KResult<Shape> {
            b::cylinder_surface(o[0], o[1], o[2], d[0], d[1], d[2], r, h0, h1).map(Shape).map_err(err)
        }
        /// mode: 0 Frenet corrigido, 1 fixo, 2 Frenet
        pub fn sweep_surface(profile: &[[f64; 3]], spine: &[[f64; 3]], mode: u8) -> KResult<Shape> {
            let p: Vec<f64> = profile.iter().flatten().copied().collect();
            let s: Vec<f64> = spine.iter().flatten().copied().collect();
            b::sweep_surface(&p, &s, mode).map(Shape).map_err(err)
        }
        pub fn loft_surface(sections: &[Vec<[f64; 3]>], ruled: bool, closed: bool, guide: Option<&[[f64; 3]]>) -> KResult<Shape> {
            let (mut flat, mut ends) = (vec![], vec![]);
            for c in sections {
                flat.extend(c.iter().flatten().copied());
                ends.push((flat.len() / 3) as u32);
            }
            let g: Vec<f64> = guide.unwrap_or(&[]).iter().flatten().copied().collect();
            b::loft_surface(&flat, &ends, ruled, closed, &g).map(Shape).map_err(err)
        }
        pub fn sewn(&self, tol: f64) -> KResult<Shape> {
            b::sew_shapes(&self.0, tol).map(Shape).map_err(err)
        }
        pub fn healed(&self, tol: f64) -> KResult<Shape> {
            b::heal_shape(&self.0, tol).map(Shape).map_err(err)
        }
        /// Divide pela ferramenta e fica com o lado da normal (`keep_pos`) ou o oposto.
        pub fn split_side(&self, tool: &Shape, keep_pos: bool, extend: bool) -> KResult<Shape> {
            b::split_side(&self.0, &tool.0, keep_pos, extend).map(Shape).map_err(err)
        }
        pub fn nth_face(&self, i: u32) -> KResult<Shape> {
            b::nth_face(&self.0, i).map(Shape).map_err(err)
        }
        pub fn untrimmed(&self) -> KResult<Shape> {
            b::untrim_face(&self.0).map(Shape).map_err(err)
        }
        /// `curvature`: prolonga mantendo a curvatura (mesma superfície); senão, tangente (linear).
        pub fn extrapolated(&self, near: [f64; 3], len: f64, curvature: bool) -> KResult<Shape> {
            b::extrapolate_face(&self.0, near[0], near[1], near[2], len, if curvature { 2 } else { 1 }).map(Shape).map_err(err)
        }
        /// Escala independente nos eixos do sistema `f` (afinidade).
        pub fn affinity(&self, f: Frame, k: [f64; 3]) -> KResult<Shape> {
            b::affinity(&self.0, f.origin[0], f.origin[1], f.origin[2], f.x[0], f.x[1], f.x[2], f.normal[0], f.normal[1], f.normal[2], k[0], k[1], k[2]).map(Shape).map_err(err)
        }
        fn split_pts(flat: Vec<f64>, ends: Vec<u32>) -> Vec<Vec<[f64; 3]>> {
            let mut out = vec![];
            let mut s = 0usize;
            for e in ends {
                out.push((s..e as usize).map(|i| [flat[3 * i], flat[3 * i + 1], flat[3 * i + 2]]).collect());
                s = e as usize;
            }
            out
        }
        /// Bordas livres como polilinhas.
        pub fn boundary(&self) -> KResult<Vec<Vec<[f64; 3]>>> {
            let mut ends = vec![];
            let flat = b::boundary_pts(&self.0, &mut ends).map_err(err)?;
            Ok(Self::split_pts(flat, ends))
        }
        /// Curvas de interseção com outra forma.
        pub fn section(&self, other: &Shape) -> KResult<Vec<Vec<[f64; 3]>>> {
            let mut ends = vec![];
            let flat = b::section_pts(&self.0, &other.0, &mut ends).map_err(err)?;
            Ok(Self::split_pts(flat, ends))
        }
        pub fn thicken(&self, t1: f64, t2: f64) -> KResult<Shape> {
            b::thicken(&self.0, t1, t2).map(Shape).map_err(err)
        }
        pub fn closed_solid(&self, tol: f64) -> KResult<Shape> {
            b::close_surface(&self.0, tol).map(Shape).map_err(err)
        }
        /// Volumes fechados formados pelas formas (compound), sem os que contêm pontos de `exclude`.
        pub fn volume_cells(&self, exclude: &[[f64; 3]]) -> KResult<Shape> {
            let e: Vec<f64> = exclude.iter().flatten().copied().collect();
            b::volume_cells(&self.0, &e).map(Shape).map_err(err)
        }
        pub fn from_mesh(tris: &[f64], solid: bool) -> KResult<Shape> {
            b::mesh_shape(tris, solid).map(Shape).map_err(err)
        }
        pub fn is_solid(&self) -> bool {
            b::is_solid(&self.0)
        }
        pub fn to_brep(&self) -> KResult<String> {
            b::to_brep(&self.0).map_err(err)
        }
        /// Projeção com linhas ocultas: (polilinha 2D, tipo) — 0 visível, 1 tangente, 2 contorno, 3 oculta.
        pub fn hlr(&self, toward_viewer: [f64; 3], x: [f64; 3], hidden: bool) -> KResult<Vec<(Vec<[f64; 2]>, u8)>> {
            let (mut ends, mut kinds) = (vec![], vec![]);
            let f = b::hlr_project(&self.0, toward_viewer[0], toward_viewer[1], toward_viewer[2], x[0], x[1], x[2], hidden, &mut ends, &mut kinds).map_err(err)?;
            let mut out = vec![];
            let mut s = 0usize;
            for (e, k) in ends.iter().zip(kinds) {
                out.push(((s..*e as usize).map(|i| [f[2 * i], f[2 * i + 1]]).collect(), k));
                s = *e as usize;
            }
            Ok(out)
        }
        /// Contornos da seção pelo plano (o, n) — polilinhas 3D fechadas.
        pub fn section_loops(&self, o: [f64; 3], n: [f64; 3], half: f64) -> KResult<Vec<Vec<[f64; 3]>>> {
            let mut ends = vec![];
            let f = b::section_loops(&self.0, o[0], o[1], o[2], n[0], n[1], n[2], half, &mut ends).map_err(err)?;
            let mut out = vec![];
            let mut s = 0usize;
            for e in ends {
                out.push((s..e as usize).map(|i| [f[3 * i], f[3 * i + 1], f[3 * i + 2]]).collect());
                s = e as usize;
            }
            Ok(out)
        }
        /// Aba de chapa na aresta perto de `p` (ou a tira planificada, se `flat`).
        pub fn sm_flange(&self, p: [f64; 3], f: SheetFlange) -> KResult<(Shape, SheetEdgeInfo)> {
            let mut info = vec![];
            let s = b::sm_flange(&self.0, p[0], p[1], p[2], f.t, f.r, f.angle, f.length, f.off1, f.off2, f.flip, f.flat, f.k, &mut info).map_err(err)?;
            let i = SheetEdgeInfo {
                length: info[0],
                thickness: info[1],
                normal: [info[2], info[3], info[4]],
                out: [info[5], info[6], info[7]],
                ends: [[info[8], info[9], info[10]], [info[11], info[12], info[13]]],
            };
            Ok((Shape(s), i))
        }
        /// Perfil plano deslocado para fora (d > 0) ou para dentro.
        pub fn offset_face(&self, d: f64) -> KResult<Shape> {
            b::offset_face(&self.0, d).map(Shape).map_err(err)
        }
        /// Caixa: canto `o`, eixo `x` (lx), normal `z` (lz), y = z × x (ly).
        pub fn box_tool(o: [f64; 3], x: [f64; 3], z: [f64; 3], l: [f64; 3]) -> KResult<Shape> {
            b::box_tool(o[0], o[1], o[2], x[0], x[1], x[2], z[0], z[1], z[2], l[0], l[1], l[2]).map(Shape).map_err(err)
        }
        /// Prisma de um polígono plano ao longo de `v`.
        pub fn poly_prism(pts: &[[f64; 3]], v: [f64; 3]) -> KResult<Shape> {
            let f: Vec<f64> = pts.iter().flatten().copied().collect();
            b::poly_prism(&f, v[0], v[1], v[2]).map(Shape).map_err(err)
        }
        pub fn from_brep(text: &str) -> KResult<Shape> {
            b::from_brep(text).map(Shape).map_err(err)
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
                edge_smooth: m.edge_smooth,
                vertices: m.vertices,
                edge_faces: pairs(&m.edge_faces),
                face_info: face_infos(&m.face_info),
            })
        }
        /// Índice (na ordem das faces da malha) da face mais próxima do ponto, se estiver a no máximo `limit`.
        pub fn face_index(&self, p: [f64; 3], limit: f64) -> KResult<Option<usize>> {
            let i = b::face_index(&self.0, p[0], p[1], p[2], limit).map_err(err)?;
            Ok((i >= 0).then_some(i as usize))
        }
        /// Faces e arestas com assinatura geométrica e vizinhança, sem tesselar.
        pub fn topology(&self) -> KResult<Topology> {
            let (mut fi, mut em, mut ef) = (vec![], vec![], vec![]);
            b::topology(&self.0, &mut fi, &mut em, &mut ef).map_err(err)?;
            Ok(Topology { faces: face_infos(&fi), edge_mids: em.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect(), edge_faces: pairs(&ef) })
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
        pub fn extrude_roles(&self, _: [f64; 3], _: f64, _: f64) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn revolve_roles(&self, _: [f64; 3], _: [f64; 3], _: f64, _: f64) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn boolean_tagged_src(&self, _: &Shape, _: Bool, _: &[i32]) -> KResult<(Shape, Vec<i32>, Vec<i32>)> {
            no()
        }
        pub fn topology(&self) -> KResult<Topology> {
            no()
        }
        pub fn face_index(&self, _: [f64; 3], _: f64) -> KResult<Option<usize>> {
            no()
        }
        pub fn boolean(&self, _: &Shape, _: Bool) -> KResult<Shape> {
            no()
        }
        pub fn hole_tool_ext(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: HoleShape, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn ray_hit(&self, _: [f64; 3], _: [f64; 3]) -> KResult<Option<([f64; 3], f64)>> {
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
        pub fn offset_surface_join(&self, _: f64, _: u8) -> KResult<Shape> {
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
        pub fn sphere_surface(_: [f64; 3], _: f64) -> KResult<Shape> {
            no()
        }
        pub fn cylinder_surface(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn sweep_surface(_: &[[f64; 3]], _: &[[f64; 3]], _: u8) -> KResult<Shape> {
            no()
        }
        pub fn loft_surface(_: &[Vec<[f64; 3]>], _: bool, _: bool, _: Option<&[[f64; 3]]>) -> KResult<Shape> {
            no()
        }
        pub fn sewn(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn healed(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn split_side(&self, _: &Shape, _: bool, _: bool) -> KResult<Shape> {
            no()
        }
        pub fn nth_face(&self, _: u32) -> KResult<Shape> {
            no()
        }
        pub fn untrimmed(&self) -> KResult<Shape> {
            no()
        }
        pub fn extrapolated(&self, _: [f64; 3], _: f64, _: bool) -> KResult<Shape> {
            no()
        }
        pub fn affinity(&self, _: Frame, _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn boundary(&self) -> KResult<Vec<Vec<[f64; 3]>>> {
            no()
        }
        pub fn section(&self, _: &Shape) -> KResult<Vec<Vec<[f64; 3]>>> {
            no()
        }
        pub fn thicken(&self, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn closed_solid(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn volume_cells(&self, _: &[[f64; 3]]) -> KResult<Shape> {
            no()
        }
        pub fn from_mesh(_: &[f64], _: bool) -> KResult<Shape> {
            no()
        }
        pub fn is_solid(&self) -> bool {
            false
        }
        pub fn to_brep(&self) -> KResult<String> {
            no()
        }
        pub fn hlr(&self, _: [f64; 3], _: [f64; 3], _: bool) -> KResult<Vec<(Vec<[f64; 2]>, u8)>> {
            no()
        }
        pub fn section_loops(&self, _: [f64; 3], _: [f64; 3], _: f64) -> KResult<Vec<Vec<[f64; 3]>>> {
            no()
        }
        pub fn sm_flange(&self, _: [f64; 3], _: SheetFlange) -> KResult<(Shape, SheetEdgeInfo)> {
            no()
        }
        pub fn offset_face(&self, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn box_tool(_: [f64; 3], _: [f64; 3], _: [f64; 3], _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn poly_prism(_: &[[f64; 3]], _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn from_brep(_: &str) -> KResult<Shape> {
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
    fn extrusao_devolve_o_papel_de_cada_face() {
        let caixa = Shape::face(&XY, &[rect(40., 20.)]).unwrap();
        let (s, roles) = caixa.extrude_roles([0., 0., 1.], 10., 0.).unwrap();
        assert_eq!(roles.len() as u32, s.face_count());
        let mut ordenados = roles.clone();
        ordenados.sort();
        // 0 = tampa inicial, 1 = tampa final, 2..=5 = uma lateral por aresta do perfil
        assert_eq!(ordenados, vec![0, 1, 2, 3, 4, 5]);
        let topo = s.topology().unwrap();
        assert_eq!(topo.faces.len(), roles.len());
        let normal_de = |papel: i32| topo.faces[roles.iter().position(|r| *r == papel).unwrap()].dir;
        assert_eq!(normal_de(1), [0., 0., 1.], "tampa final fica em +z");
        assert_eq!(normal_de(0), [0., 0., -1.], "tampa inicial fica em -z");
        // cada aresta entre a tampa final e uma lateral tem exatamente essas duas faces como vizinhas
        let (i_topo, i_lat) = (roles.iter().position(|r| *r == 1).unwrap() as u32, roles.iter().position(|r| *r == 2).unwrap() as u32);
        assert!(topo.edge_faces.iter().any(|p| p.contains(&i_topo) && p.contains(&i_lat)));
        let m = s.mesh(0.1).unwrap();
        assert_eq!(m.edge_faces.len(), m.edge_ranges.len());
        assert_eq!(m.face_info.len(), m.face_ranges.len());
        // a malha e a topologia concordam
        assert_eq!(m.edge_faces, topo.edge_faces);
        // com a mudança do perfil, os papéis continuam nos mesmos lados
        let (s2, r2) = Shape::face(&XY, &[rect(60., 20.)]).unwrap().extrude_roles([0., 0., 1.], 10., 0.).unwrap();
        let t2 = s2.topology().unwrap();
        let normal2 = |papel: i32| t2.faces[r2.iter().position(|r| *r == papel).unwrap()].dir;
        for papel in 0..6 {
            assert_eq!(normal_de(papel), normal2(papel), "papel {papel}");
        }
    }

    #[test]
    fn revolucao_devolve_papeis_e_booleana_leva_os_da_ferramenta() {
        let l = rect(10., 20.).into_iter().map(|s| match s {
            Seg2::Line(a, b) => Seg2::Line([a[0] + 10., a[1]], [b[0] + 10., b[1]]),
            o => o,
        });
        // perfil afastado do eixo (um tubo): quatro arestas, quatro faces de revolução, sem tampas
        let f = Shape::face(&XY, &[l.collect()]).unwrap();
        let (r, roles) = f.revolve_roles([0.; 3], [0., 1., 0.], 360., 0.).unwrap();
        assert_eq!(roles.len() as u32, r.face_count());
        let mut ordenados = roles.clone();
        ordenados.sort();
        assert_eq!(ordenados, vec![2, 3, 4, 5], "uma face por aresta do perfil: {roles:?}");
        // booleana: as faces da parede do corte vêm da ferramenta e mantêm o papel dela
        let caixa = Shape::face(&XY, &[rect(40., 40.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let (ferr, rf) = Shape::face(&XY, &[rect(10., 10.)]).unwrap().extrude_roles([0., 0., 1.], 5., 0.).unwrap();
        let tags: Vec<i32> = (0..caixa.face_count() as i32).collect();
        let (c, out, src) = caixa.boolean_tagged_src(&ferr, Bool::Cut, &tags).unwrap();
        assert_eq!(out.len(), src.len());
        let paredes: Vec<i32> = (0..out.len()).filter(|i| out[*i] < 0 && src[*i] >= 0).map(|i| rf[src[i] as usize]).collect();
        assert_eq!(c.face_count() as usize, out.len());
        assert!(paredes.len() >= 4, "as laterais da ferramenta viram parede: {paredes:?}");
    }

    #[test]
    fn aresta_inexistente_e_erro_nao_descarte_silencioso() {
        let caixa = Shape::face(&XY, &[rect(20., 20.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let tags: Vec<i32> = (0..caixa.face_count() as i32).collect();
        // uma aresta existe, a outra está longe de qualquer aresta
        let r = caixa.edges_tagged(1., &[[10., 0., 10.], [3., 3., 3.]], 0.5, false, &tags);
        let msg = r.err().expect("deveria falhar").0;
        assert!(msg.contains("não existe"), "{msg}");
        // face fora do alcance: erro em vez de pegar a mais próxima
        let r = caixa.shell_tagged(&[[0., 0., 500.]], 1., false, &tags);
        assert!(r.err().expect("deveria falhar").0.contains("não existe mais"));
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
