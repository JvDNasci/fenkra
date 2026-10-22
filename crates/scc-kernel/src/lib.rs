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

        pub fn mesh(&self, deflection: f64) -> KResult<Mesh> {
            let m = b::tessellate(&self.0, deflection).map_err(err)?;
            Ok(Mesh { positions: m.positions, normals: m.normals, indices: m.indices, edges: m.edges })
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
