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
        /// início (em segmentos de `edges`) de cada aresta, na ordem do kernel
        edge_ranges: Vec<u32>,
        /// ponto médio (xyz) de cada aresta — assinatura para referência persistente
        edge_mids: Vec<f32>,
        /// posição (xyz) de cada vértice
        vertices: Vec<f32>,
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
        fn fillet_sel_t(s: &Shape, r: f64, mids: &[f64], tol: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        fn chamfer_sel_t(s: &Shape, d: f64, mids: &[f64], tol: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        /// Ferramenta de rosca modelada (sulco helicoidal em V) a partir de `o`, ao longo de `d`.
        fn thread_tool(ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64, major_d: f64, pitch: f64, length: f64, angle_deg: f64)
            -> Result<UniquePtr<Shape>>;
        /// Extrusão do perfil na direção d até o plano (o, n), com comprimento máximo `len`.
        fn extrude_until(face: &Shape, dx: f64, dy: f64, dz: f64, len: f64, ox: f64, oy: f64, oz: f64, nx: f64, ny: f64, nz: f64)
            -> Result<UniquePtr<Shape>>;
        fn chamfer_all_t(s: &Shape, d: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;

        /// kind: 0 translação (dx,dy,dz), 1 rotação (o, d, graus), 2 espelho (o, normal), 3 escala (o, fator)
        fn transform_shape(s: &Shape, kind: u8, p: &[f64]) -> Result<UniquePtr<Shape>>;
        fn transform_t(s: &Shape, kind: u8, p: &[f64], tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        /// Varre o perfil (face) ao longo da polilinha `spine` (xyz…); modo 0 Frenet corrigido, 1 fixo, 2 Frenet.
        fn sweep_tool(face: &Shape, spine: &[f64], mode: u8) -> Result<UniquePtr<Shape>>;
        /// Multi-seções pelas faces do composto (na ordem).
        fn loft_tool(sections: &Shape, ruled: bool, closed: bool) -> Result<UniquePtr<Shape>>;
        /// 0 caixa, 1 cilindro, 2 esfera, 3 cone, 4 toro, 5 cunha, 6 prisma poligonal
        fn primitive(kind: u8, frame: &Frame, p: &[f64]) -> Result<UniquePtr<Shape>>;
        fn shell_t(s: &Shape, pts: &[f64], thickness: f64, outward: bool, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        fn draft_t(
            s: &Shape, pts: &[f64], dx: f64, dy: f64, dz: f64, angle_deg: f64, ox: f64, oy: f64, oz: f64, nx: f64, ny: f64, nz: f64,
            tags: &[i32], out: &mut Vec<i32>,
        ) -> Result<UniquePtr<Shape>>;
        /// Prismas das faces planas escolhidas, deslocados ao longo da normal externa (valor negativo = para dentro).
        fn faces_offset_tool(s: &Shape, pts: &[f64], value: f64) -> Result<UniquePtr<Shape>>;
        fn extrude_taper(face: &Shape, dx: f64, dy: f64, dz: f64, len: f64, angle_deg: f64) -> Result<UniquePtr<Shape>>;
        /// Perfil de parede fina (espessura t centrada no contorno).
        fn thin_profile(face: &Shape, t: f64) -> Result<UniquePtr<Shape>>;
        /// Face engrossada a partir de segmentos (pares de pontos xyz) no plano de normal n.
        fn thick_segments(segs: &[f64], nx: f64, ny: f64, nz: f64, t: f64) -> Result<UniquePtr<Shape>>;
        /// modo 0 filete variável (r1→r2), 1 chanfro duas distâncias, 2 chanfro distância + ângulo
        fn edges_mode_t(s: &Shape, mode: u8, v1: f64, v2: f64, mids: &[f64], tol: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        /// [ox, oy, oz, dx, dy, dz, raio, h0, h1, interna]
        fn cylinder_at(s: &Shape, px: f64, py: f64, pz: f64) -> Result<Vec<f64>>;
        fn thread_tool_ext(ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64, major_d: f64, pitch: f64, length: f64, angle_deg: f64)
            -> Result<UniquePtr<Shape>>;
        /// Cópia da face mais próxima do ponto.
        fn face_copy(s: &Shape, px: f64, py: f64, pz: f64) -> Result<UniquePtr<Shape>>;
        fn offset_surface(s: &Shape, d: f64) -> Result<UniquePtr<Shape>>;
        /// Superfície que preenche o contorno das curvas (polilinhas; `ends` = fim de cada uma, em pontos).
        fn fill_surface(flat: &[f64], ends: &[u32]) -> Result<UniquePtr<Shape>>;
        fn extrude_curve(pts: &[f64], dx: f64, dy: f64, dz: f64, len: f64) -> Result<UniquePtr<Shape>>;
        fn revolve_curve(pts: &[f64], ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64, angle_deg: f64) -> Result<UniquePtr<Shape>>;
        /// [qx, qy, qz, nx, ny, nz]: ponto mais próximo na forma e normal da face
        fn surface_near(s: &Shape, px: f64, py: f64, pz: f64) -> Result<Vec<f64>>;
        /// [área, tipo, nx, ny, nz, ox, oy, oz, raio]
        fn face_props(s: &Shape, px: f64, py: f64, pz: f64) -> Result<Vec<f64>>;
        fn extrude_to_next(face: &Shape, dx: f64, dy: f64, dz: f64, len: f64, body: &Shape) -> Result<UniquePtr<Shape>>;
        fn clip_until(tool: &Shape, cutter: &Shape, px: f64, py: f64, pz: f64, extend: bool) -> Result<UniquePtr<Shape>>;
        fn plane_face(ox: f64, oy: f64, oz: f64, nx: f64, ny: f64, nz: f64, half: f64) -> Result<UniquePtr<Shape>>;
        fn same_face(s: &Shape, ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> Result<bool>;
        fn full_round_t(s: &Shape, mx: f64, my: f64, mz: f64, width: f64, tags: &[i32], out: &mut Vec<i32>) -> Result<UniquePtr<Shape>>;
        fn face_face_fillet_t(
            s: &Shape, ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64, r: f64, tags: &[i32], out: &mut Vec<i32>,
        ) -> Result<UniquePtr<Shape>>;

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
