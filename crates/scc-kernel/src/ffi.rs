//! Ponte Rust ⇄ C++ (cxx). Mantenha tipos simples: o C++ é só uma fachada sobre o OCCT.

#[cxx::bridge(namespace = "scc")]
pub mod bridge {
    /// Sistema de coordenadas de um plano de sketch.
    #[derive(Clone, Copy, Debug)]
    struct Frame {
        ox: f64,
        oy: f64,
        oz: f64,
        xx: f64,
        xy: f64,
        xz: f64,
        nx: f64,
        ny: f64,
        nz: f64,
    }

    /// Segmento 2D de um perfil. kind: 0 = linha (p1→p2), 1 = arco por 3 pontos (p1, p2 meio, p3),
    /// 2 = círculo (centro p1, raio x2).
    #[derive(Clone, Copy, Debug)]
    struct Seg {
        kind: u8,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    }

    struct MeshData {
        /// xyz por vértice
        positions: Vec<f32>,
        normals: Vec<f32>,
        indices: Vec<u32>,
        /// pares de pontos (segmentos de linha) das arestas
        edges: Vec<f32>,
        /// início (em `indices`) dos triângulos de cada face, na ordem das etiquetas
        face_ranges: Vec<u32>,
    }

    unsafe extern "C++" {
        include!("kernel.h");

        type Shape;

        fn shape_clone(s: &Shape) -> UniquePtr<Shape>;
        fn shape_is_null(s: &Shape) -> bool;
        fn shape_empty() -> UniquePtr<Shape>;

        /// Perfil plano a partir de laços fechados; `loop_ends[i]` = índice final (exclusivo) do laço i.
        fn make_face(frame: &Frame, segs: &[Seg], loop_ends: &[u32]) -> Result<UniquePtr<Shape>>;
        /// Extrusão do perfil ao longo da normal do frame, de -back até +front.
        fn extrude(face: &Shape, dx: f64, dy: f64, dz: f64, front: f64, back: f64) -> Result<UniquePtr<Shape>>;
        fn revolve(face: &Shape, ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64, angle_deg: f64, start_deg: f64)
            -> Result<UniquePtr<Shape>>;
        /// op: 0 = unir, 1 = subtrair, 2 = interseção
        fn boolean(a: &Shape, b: &Shape, op: u8) -> Result<UniquePtr<Shape>>;
        /// Ferramenta de furo (sólido a subtrair) com eixo (o, d) entrando no material.
        fn hole_tool(
            ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64,
            diameter: f64, depth: f64, kind: u8, d2: f64, p2: f64,
        ) -> Result<UniquePtr<Shape>>;
        fn fillet_all(s: &Shape, radius: f64) -> Result<UniquePtr<Shape>>;
        fn chamfer_all(s: &Shape, dist: f64) -> Result<UniquePtr<Shape>>;
        fn compound(a: &Shape, b: &Shape) -> UniquePtr<Shape>;
        /// Versões com etiquetas: `tags` = dono de cada face de `a`; `out` = dono de cada face do
        /// resultado (-1 = face criada pela operação).
        fn boolean_t(a: &Shape, b: &Shape, op: u8, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        fn fillet_all_t(s: &Shape, r: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        fn chamfer_all_t(s: &Shape, d: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;

        fn tessellate(s: &Shape, deflection: f64) -> Result<MeshData>;
        /// [xmin, ymin, zmin, xmax, ymax, zmax]
        fn bounding_box(s: &Shape) -> Vec<f64>;
        /// [volume, área, cx, cy, cz]
        fn mass_props(s: &Shape) -> Vec<f64>;
        fn count_faces(s: &Shape) -> u32;
        fn export_step(s: &Shape, path: &str) -> Result<()>;
        fn export_stl(s: &Shape, path: &str) -> Result<()>;
        fn import_step(path: &str) -> Result<UniquePtr<Shape>>;
        fn occt_version() -> String;
    }
}
