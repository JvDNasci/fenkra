// SPDX-License-Identifier: LGPL-2.1-or-later
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

/// O que um arquivo STEP declara sobre si: esquema do cabeçalho e unidade de comprimento.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InfoStep {
    pub esquema: String,
    pub unidades: String,
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
    /// Elipse: centro, vetor do semieixo maior, semieixo menor, parâmetros inicial e final.
    Ellipse([f64; 2], [f64; 2], f64, f64, f64),
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

/// Uma amostra da junção de duas superfícies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmostraConexao {
    pub ponto: [f64; 3],
    /// distância até a outra superfície, em mm
    pub folga: f64,
    /// quanto as superfícies dobram ao cruzar a junção, em graus: zero é tangente
    pub angulo: f64,
    /// curvatura de cada lado na direção que atravessa a borda, em 1/mm, com a mesma referência de sinal
    pub curvatura: [f64; 2],
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    /// curvaturas principais (máxima, mínima) de cada vértice, em 1/mm, medidas na superfície exata:
    /// não mudam com a malha nem com o zoom. Vazia em malha que não veio do kernel.
    pub curvature: Vec<f32>,
    pub indices: Vec<u32>,
    pub edges: Vec<f32>,
    /// início (em `indices`) dos triângulos de cada face
    pub face_ranges: Vec<u32>,
    /// início (em segmentos de `edges`) de cada aresta
    pub edge_ranges: Vec<u32>,
    /// ponto médio (xyz) de cada aresta
    pub edge_mids: Vec<f32>,
    /// fórmula de cada aresta, em `EDGE_GEOM` números (ver `Mesh::edge_formula`); em precisão dupla,
    /// porque serve para projetar no sketch, não para desenhar
    pub edge_geom: Vec<f64>,
    /// 1 por aresta suave (tangente entre as faces vizinhas), 0 por aresta viva
    pub edge_smooth: Vec<u8>,
    /// posição (xyz) de cada vértice
    pub vertices: Vec<f32>,
    /// as duas faces vizinhas de cada aresta (índices na ordem de `face_ranges`; `NO_FACE` = nenhuma)
    pub edge_faces: Vec<[u32; 2]>,
    /// assinatura de cada face, na ordem de `face_ranges`
    pub face_info: Vec<FaceInfo>,
}

/// Números por aresta em `Mesh::edge_geom`.
pub const EDGE_GEOM: usize = 12;

/// Fórmula de uma aresta. Cônica: `P(t) = c + u cos t + v sin t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeFormula {
    Line { a: [f64; 3], b: [f64; 3] },
    Conic { c: [f64; 3], u: [f64; 3], v: [f64; 3], t0: f64, t1: f64 },
}

impl Mesh {
    /// Fórmula da aresta `e`, se ela for reta, círculo ou elipse.
    pub fn edge_formula(&self, e: usize) -> Option<EdgeFormula> {
        let g = self.edge_geom.get(e * EDGE_GEOM..(e + 1) * EDGE_GEOM)?;
        let v3 = |i: usize| [g[i], g[i + 1], g[i + 2]];
        match g[0] as i32 {
            1 => Some(EdgeFormula::Line { a: v3(1), b: v3(4) }),
            2 => Some(EdgeFormula::Conic { c: v3(1), u: v3(4), v: v3(7), t0: g[10], t1: g[11] }),
            _ => None,
        }
    }

    /// Memória ocupada pelos dados da malha, em bytes.
    pub fn bytes(&self) -> usize {
        4 * (self.positions.len() + self.normals.len() + self.indices.len() + self.edges.len())
            + 4 * (self.face_ranges.len() + self.edge_ranges.len() + self.edge_mids.len() + self.vertices.len())
            + self.edge_smooth.len()
            + std::mem::size_of::<[u32; 2]>() * self.edge_faces.len()
            + std::mem::size_of::<FaceInfo>() * self.face_info.len()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MassProps {
    pub volume: f64,
    pub area: f64,
    pub center: [f64; 3],
}

/// Inércia do volume com densidade 1, em torno do centro de massa (unidades de mm^5; multiplique
/// pela densidade em g/mm^3 para obter g·mm^2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InertiaProps {
    /// matriz simétrica [xx, yy, zz, xy, xz, yz]
    pub matrix: [f64; 6],
    /// momentos principais, em ordem crescente do kernel
    pub principal: [f64; 3],
    /// eixo principal de cada momento (vetor unitário)
    pub axes: [[f64; 3]; 3],
}

/// Relatório de uma junção costurada.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SewReport {
    /// pedaços que continuam separados
    pub domains: u32,
    pub free_edges: u32,
    /// maior folga que a costura fechou (mm)
    pub closed_gap: f64,
    /// maior distância entre domínios que ficaram separados (mm)
    pub open_gap: f64,
}

/// Ângulo máximo entre normais vizinhas da malha quando ninguém pede outro (20°, em radianos).
pub const ANGULO_PADRAO: f64 = 0.35;

#[cfg(occt)]
mod ffi;

pub mod xde;
pub mod pmi_xde;
pub use pmi_xde::{AlvoDatumXde, AlvoXde, AnotacaoXde, ApresentacaoXde, DadoXde, VistaXde, MaterialXde, ModToleranciaXde, ModZonaXde, TipoDimensaoXde, TipoToleranciaXde, ZonaXde};
pub use xde::{DefeitoForma, EsquemaStep, InfoReparo, InstanciaXde, ModeloXde, OpcoesGravarIges, OpcoesGravarXde, OpcoesLerXde, ProdutoXde, TipoDefeito, UnidadeStep, IDENTIDADE};

#[cfg(occt)]
mod imp {
    use super::*;
    use cxx::UniquePtr;
    use ffi::bridge as b;

    pub const AVAILABLE: bool = true;

    /// B393: a projeção exata de linhas ocultas do OpenCASCADE guarda estado global. Duas ao mesmo
    /// tempo no processo (a vista de uma peça na interface e a de uma montagem na thread de
    /// projeção, por exemplo) saem vazias ou com linhas a menos, sem erro. Uma por vez.
    static HLR_EXATA: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn trava_hlr() -> std::sync::MutexGuard<'static, ()> {
        HLR_EXATA.lock().unwrap_or_else(|e| e.into_inner())
    }

    // Cada trabalho tem a sua forma e o seu resultado e só é usado por uma thread de cada vez.
    struct Trabalho(UniquePtr<b::HlrTrabalho>);
    unsafe impl Send for Trabalho {}

    /// Linhas ocultas de várias vistas (B393). `poligonal = Some(desvio)` projeta a malha (bem
    /// mais rápido, arestas retas; serve de prévia), uma thread por vista, cada vista além da
    /// primeira com a sua cópia da forma. A projeção exata (`None`) roda uma vista depois da outra:
    /// o algoritmo exato do OpenCASCADE guarda estado global, e duas threads ao mesmo tempo
    /// devolvem vistas vazias ou com linhas a menos, sem erro (visto no kart: 2067, 2063, 2068 e 0
    /// linhas para quatro cópias da mesma vista). Para a exata em paralelo, use processos.
    pub fn hlr_varias(vistas: &[VistaHlr], poligonal: Option<f64>) -> Vec<KResult<Vec<(Vec<[f64; 2]>, u8)>>> {
        let desvio = poligonal.unwrap_or(0.);
        let mut trabalhos: Vec<KResult<Trabalho>> = vec![];
        for (i, v) in vistas.iter().enumerate() {
            // `Shape::clone` divide a topologia: comparar os `Shape` não diz se duas vistas leem a
            // mesma forma, e as linhas ocultas de duas threads na mesma topologia saem vazias
            // (visto no kart). Toda vista depois da primeira projeta uma cópia própria.
            let copiar = poligonal.is_some() && i > 0;
            let t = b::hlr_trabalho_novo(v.forma.ponte(), v.para[0], v.para[1], v.para[2], v.x[0], v.x[1], v.x[2], v.ocultas, copiar, desvio);
            trabalhos.push(t.map(Trabalho).map_err(err));
        }
        let baixa = vistas.first().is_some_and(|v| v.prioridade_baixa);
        if poligonal.is_none() {
            let _uma = trava_hlr();
            for t in trabalhos.iter_mut().filter_map(|t| t.as_mut().ok()) {
                if let Some(p) = t.0.as_mut() {
                    b::hlr_trabalho_rodar(p);
                }
            }
        } else {
            std::thread::scope(|sc| {
            for t in trabalhos.iter_mut().filter_map(|t| t.as_mut().ok()) {
                sc.spawn(move || {
                    if baixa {
                        super::reduzir_prioridade_da_thread();
                    }
                    if let Some(p) = t.0.as_mut() {
                        b::hlr_trabalho_rodar(p);
                    }
                });
            }
            });
        }
        trabalhos
            .into_iter()
            .map(|t| {
                let t = t?;
                let (mut ends, mut kinds) = (vec![], vec![]);
                let f = b::hlr_trabalho_resultado(&t.0, &mut ends, &mut kinds).map_err(err)?;
                let mut out = vec![];
                let mut s = 0usize;
                for (e, k) in ends.iter().zip(kinds) {
                    out.push(((s..*e as usize).map(|i| [f[2 * i], f[2 * i + 1]]).collect(), k));
                    s = *e as usize;
                }
                Ok(out)
            })
            .collect()
    }

    /// Forma lida por várias threads ao mesmo tempo, só para copiar ou como entrada da booleana não
    /// destrutiva. Vale enquanto quem empresta não mexe nela, o que o empréstimo garante; a contagem
    /// de referências do OCCT é atômica.
    struct SoLeitura<'a>(&'a b::Shape);
    unsafe impl Sync for SoLeitura<'_> {}

    fn repartir_n(threads: usize, n: usize) -> usize {
        if threads == 0 { std::thread::available_parallelism().map_or(4, |n| n.get()) } else { threads }.min(n).max(1)
    }

    /// `f(i)` para i em 0..n, em `threads` threads que pegam o próximo índice livre (os itens têm
    /// custos muito diferentes). Devolve na ordem dos índices.
    fn repartir<R: Send>(n: usize, threads: usize, f: impl Fn(usize) -> R + Sync) -> Vec<R> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let t = repartir_n(threads, n);
        if t == 1 {
            return (0..n).map(f).collect();
        }
        let proximo = AtomicUsize::new(0);
        let mut feitos: Vec<(usize, R)> = std::thread::scope(|sc| {
            let ts: Vec<_> = (0..t)
                .map(|_| {
                    sc.spawn(|| {
                        let mut v = vec![];
                        loop {
                            let i = proximo.fetch_add(1, Ordering::Relaxed);
                            if i >= n {
                                break;
                            }
                            v.push((i, f(i)));
                        }
                        v
                    })
                })
                .collect();
            ts.into_iter().flat_map(|t| t.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
        });
        feitos.sort_by_key(|x| x.0);
        feitos.into_iter().map(|x| x.1).collect()
    }

    pub struct Shape(UniquePtr<b::Shape>);

    impl Shape {
        pub(crate) fn ponte(&self) -> &b::Shape {
            &self.0
        }
        pub(crate) fn da_ponte(s: UniquePtr<b::Shape>) -> Shape {
            Shape(s)
        }
    }

    impl Clone for Shape {
        fn clone(&self) -> Self {
            Shape(b::shape_clone(&self.0))
        }
    }

    fn err(e: cxx::Exception) -> KernelError {
        KernelError(e.what().to_string())
    }

    fn malha_do_kernel(m: b::MeshData) -> Mesh {
        Mesh {
            positions: m.positions,
            normals: m.normals,
            curvature: m.curvature,
            indices: m.indices,
            edges: m.edges,
            face_ranges: m.face_ranges,
            edge_ranges: m.edge_ranges,
            edge_geom: m.edge_geom,
            edge_mids: m.edge_mids,
            edge_smooth: m.edge_smooth,
            vertices: m.vertices,
            edge_faces: pairs(&m.edge_faces),
            face_info: face_infos(&m.face_info),
        }
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
                        // elipse: dois registros, porque são sete números
                        Seg2::Ellipse(c, m, r, t0, t1) => {
                            segs.push(b::Seg { kind: 4, x1: c[0], y1: c[1], x2: m[0], y2: m[1], x3: *r, ..z });
                            segs.push(b::Seg { kind: 5, x1: t0.min(*t1), x2: t0.max(*t1), ..z });
                        }
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

        /// Interseção para medir interferência (B394): sem unificar as faces (o volume e a malha não
        /// mudam, e a unificação custava quase metade da análise) e sem mexer nas entradas.
        pub fn comum_medida(&self, other: &Shape) -> KResult<Shape> {
            b::comum_medida(&self.0, &other.0, false).map(Shape).map_err(err)
        }

        /// Várias [`Shape::comum_medida`] em `threads` threads (0 = uma por núcleo). `f` recebe o
        /// índice do par e o resultado na própria thread, e devolve o que interessa (volume, malha):
        /// o resultado nasce de cópias profundas das entradas e não divide nada com outra thread.
        ///
        /// Por que as cópias: a booleana em si não guarda estado global (ao contrário da projeção
        /// exata de linhas ocultas), mas uma peça entra em vários pares, e o comum de uma peça
        /// inteira dentro da outra devolve as próprias faces da entrada. Tesselar esse resultado
        /// grava a malha nessas faces, e duas threads gravariam na mesma face.
        pub fn comuns<R: Send>(pares: &[(&Shape, &Shape)], threads: usize, f: impl Fn(usize, KResult<Shape>) -> R + Sync) -> Vec<R> {
            let ps: Vec<SoLeitura> = pares.iter().flat_map(|(x, y)| [SoLeitura(&x.0), SoLeitura(&y.0)]).collect();
            let copiar = repartir_n(threads, pares.len()) > 1;
            repartir(pares.len(), threads, |i| f(i, b::comum_medida(ps[2 * i].0, ps[2 * i + 1].0, copiar).map(Shape).map_err(err)))
        }

        /// Roda `f` sobre cada forma em `threads` threads (0 = uma por núcleo), cada uma com uma cópia
        /// profunda própria da forma (geometria inclusive, sem a malha): tesselar, medir a caixa ou
        /// fazer uma booleana na cópia não toca na original nem em outra thread (B394).
        pub fn em_threads<R: Send>(formas: &[&Shape], threads: usize, f: impl Fn(usize, Shape) -> R + Sync) -> Vec<R> {
            let fs: Vec<SoLeitura> = formas.iter().map(|x| SoLeitura(&x.0)).collect();
            if repartir_n(threads, formas.len()) == 1 {
                return formas.iter().enumerate().map(|(i, s)| f(i, (*s).clone())).collect();
            }
            repartir(formas.len(), threads, |i| f(i, Shape(b::copia_profunda(fs[i].0))))
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

        /// Todos os parâmetros em que a reta (p, d) cruza a forma, em ordem crescente.
        pub fn ray_hits(&self, p: [f64; 3], d: [f64; 3]) -> KResult<Vec<f64>> {
            b::ray_hits(&self.0, p[0], p[1], p[2], d[0], d[1], d[2]).map_err(err)
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
        pub fn loft(&self, ruled: bool, closed: bool, alinhar: bool) -> KResult<Shape> {
            b::loft_tool(&self.0, ruled, closed, alinhar).map(Shape).map_err(err)
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
        /// Casca em que algumas faces têm parede própria: `proprias` traz (ponto, espessura).
        pub fn shell_faces_tagged(&self, faces: &[[f64; 3]], t: f64, outward: bool, proprias: &[([f64; 3], f64)], tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            let fpts: Vec<f64> = proprias.iter().flat_map(|x| x.0).collect();
            let vals: Vec<f64> = proprias.iter().map(|x| x.1).collect();
            let mut out = vec![];
            let s = b::shell_faces_t(&self.0, &flat, t, outward, &fpts, &vals, tags, &mut out).map_err(err)?;
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
        /// A análise do kernel aprova a topologia e a geometria do corpo.
        pub fn is_valid(&self) -> bool {
            b::shape_valid(&self.0)
        }
        /// Filete variável (B49): raio `r0` no começo da aresta, `r1` no fim e `ctrl` = pares (posição
        /// relativa 0 a 1, raio) no meio. `smooth`: spline suave pelos pontos; senão, retas entre eles.
        #[allow(clippy::too_many_arguments)]
        pub fn fillet_variable_tagged(&self, r0: f64, r1: f64, ctrl: &[[f64; 2]], smooth: bool, mids: &[[f64; 3]], tol: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = mids.iter().flatten().copied().collect();
            let c: Vec<f64> = ctrl.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = b::fillet_var_t(&self.0, r0, r1, &c, smooth, &flat, tol, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        /// Edição direta (B33): apaga as faces e as vizinhas se estendem até se encontrar.
        pub fn defeature_tagged(&self, faces: &[[f64; 3]], tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = b::defeature_t(&self.0, &flat, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        /// Edição direta (B33): desloca as faces `value` ao longo da normal (também muda raios).
        pub fn face_offset_tagged(&self, faces: &[[f64; 3]], value: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = faces.iter().flatten().copied().collect();
            let mut out = vec![];
            let s = b::face_offset_t(&self.0, &flat, value, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
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
        /// modo 0 filete variável, 1 chanfro 2 distâncias, 2 chanfro distância + ângulo, 3 filete
        /// assimétrico (aresta reta entre planos). `flips[i]` troca a face que recebe a 1ª medida.
        pub fn edges_mode_tagged(&self, mode: u8, v1: f64, v2: f64, mids: &[[f64; 3]], flips: &[bool], tol: f64, tags: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            let flat: Vec<f64> = mids.iter().flatten().copied().collect();
            let mut out = vec![];
            let fl: Vec<u8> = flips.iter().map(|x| *x as u8).collect();
            let s = b::edges_mode_t(&self.0, mode, v1, v2, &flat, &fl, tol, tags, &mut out).map_err(err)?;
            Ok((Shape(s), out))
        }
        pub fn cylinder_at(&self, p: [f64; 3]) -> KResult<Cylinder> {
            let v = b::cylinder_at(&self.0, p[0], p[1], p[2]).map_err(err)?;
            Ok(Cylinder { origin: [v[0], v[1], v[2]], dir: [v[3], v[4], v[5]], radius: v[6], h0: v[7], h1: v[8], internal: v[9] > 0.5 })
        }
        /// Ferramenta de rosca com a hélice começando na direção `xdir` (perpendicular ao eixo); `interna` escolhe sulco de furo ou de eixo.
        pub fn thread_tool_dir(origin: [f64; 3], dir: [f64; 3], xdir: [f64; 3], interna: bool, major_d: f64, pitch: f64, length: f64, angle_deg: f64) -> KResult<Shape> {
            let [ox, oy, oz] = origin;
            let [dx, dy, dz] = dir;
            let [xx, xy, xz] = xdir;
            if interna {
                b::thread_tool_x(ox, oy, oz, dx, dy, dz, major_d, pitch, length, angle_deg, xx, xy, xz).map(Shape).map_err(err)
            } else {
                b::thread_tool_ext_x(ox, oy, oz, dx, dy, dz, major_d, pitch, length, angle_deg, xx, xy, xz).map(Shape).map_err(err)
            }
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
        /// Preenchimento com condição por curva: ordem 0 (G0), 1 (G1) ou 2 (G2), alcance da tangente
        /// (`weights`, 1 = neutro), faces de apoio e pontos de passagem. Devolve o erro G0 (mm), G1 (rad)
        /// e G2 (diferença de curvatura, zero sem condição G2).
        pub fn fill_surface_x(curves: &[Vec<[f64; 3]>], orders: &[i32], weights: &[f64], support: &Shape, pts: &[[f64; 3]]) -> KResult<(Shape, f64, f64, f64)> {
            let mut flat = vec![];
            let mut ends = vec![];
            for c in curves {
                flat.extend(c.iter().flatten().copied());
                ends.push((flat.len() / 3) as u32);
            }
            let p: Vec<f64> = pts.iter().flatten().copied().collect();
            let mut errs = vec![];
            let s = b::fill_surface_x(&flat, &ends, orders, weights, &support.0, &p, &mut errs).map(Shape).map_err(err)?;
            Ok((s, errs.first().copied().unwrap_or(0.), errs.get(1).copied().unwrap_or(0.), errs.get(2).copied().unwrap_or(0.)))
        }
        /// Remendos bicúbicos soltos: 48 números por remendo (16 polos).
        pub fn bezier_faces(poles: &[f64]) -> KResult<Shape> {
            b::bezier_faces(poles).map(Shape).map_err(err)
        }
        /// Curva solta como forma (fio).
        pub fn curve_shape(pts: &[[f64; 3]]) -> KResult<Shape> {
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            b::curve_shape(&flat).map(Shape).map_err(err)
        }
        /// Relatório de uma junção já costurada.
        pub fn sew_report(&self) -> KResult<SewReport> {
            let v = b::sew_report(&self.0).map_err(err)?;
            Ok(SewReport { domains: v[0] as u32, free_edges: v[1] as u32, closed_gap: v[2], open_gap: v[3] })
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
        /// Superfície que passa por todos os pontos da grade; as linhas têm o mesmo tamanho.
        pub fn grid_surface(rows: &[Vec<[f64; 3]>]) -> KResult<Shape> {
            let nv = rows.first().map_or(0, Vec::len) as u32;
            let flat: Vec<f64> = rows.iter().flatten().flatten().copied().collect();
            b::grid_surface(&flat, rows.len() as u32, nv).map(Shape).map_err(err)
        }
        /// Amostras da junção com `other`, ver [`AmostraConexao`].
        pub fn connect_check(&self, other: &Shape, samples: u32, max_gap: f64) -> KResult<Vec<AmostraConexao>> {
            let v = b::connect_check(&self.0, &other.0, samples, max_gap).map_err(err)?;
            Ok(v.chunks_exact(7).map(|c| AmostraConexao { ponto: [c[0], c[1], c[2]], folga: c[3], angulo: c[4], curvatura: [c[5], c[6]] }).collect())
        }
        /// Distância com sinal de cada ponto até a forma: positiva do lado da normal.
        pub fn signed_distances(&self, pts: &[[f64; 3]]) -> KResult<Vec<f64>> {
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            b::signed_distances(&self.0, &flat).map_err(err)
        }
        /// Isocurvas das faces, `n` por direção.
        pub fn iso_curves(&self, n: u32) -> KResult<Vec<Vec<[f64; 3]>>> {
            let mut ends = vec![];
            let flat = b::iso_curves(&self.0, n, &mut ends).map_err(err)?;
            let mut start = 0;
            Ok(ends
                .iter()
                .map(|e| {
                    let c = flat[start * 3..*e as usize * 3].chunks_exact(3).map(|q| [q[0], q[1], q[2]]).collect();
                    start = *e as usize;
                    c
                })
                .collect())
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
        /// Pedaços depois do corte por `tool`: faces ligadas por arestas fora do corte são um pedaço só.
        pub fn regions(&self, tool: &Shape, extend: bool) -> KResult<Vec<Shape>> {
            let c = b::split_regions(&self.0, &tool.0, extend).map_err(err)?;
            (0..b::child_count(&c)).map(|i| b::nth_child(&c, i).map(Shape).map_err(err)).collect()
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
        /// Corta pelo elemento e fica com o lado que passa por `keep`.
        pub fn clip_surface(&self, cutter: &Shape, keep: [f64; 3], extend: bool) -> KResult<Shape> {
            b::clip_surface(&self.0, &cutter.0, keep[0], keep[1], keep[2], extend).map(Shape).map_err(err)
        }
        /// Polilinha da aresta mais perto do ponto.
        /// Pontos exatos em que a aresta mais perto de `p` atravessa o plano.
        pub fn edge_plane(&self, p: [f64; 3], origem: [f64; 3], normal: [f64; 3]) -> KResult<Vec<[f64; 3]>> {
            let v = b::edge_plane_pts(&self.0, p[0], p[1], p[2], origem[0], origem[1], origem[2], normal[0], normal[1], normal[2]).map_err(err)?;
            Ok(v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
        }

        pub fn edge_at(&self, p: [f64; 3]) -> KResult<Vec<[f64; 3]>> {
            let v = b::edge_pts_at(&self.0, p[0], p[1], p[2]).map_err(err)?;
            Ok(v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
        }
        /// Bordas livres, uma polilinha por aresta.
        pub fn free_edges(&self) -> KResult<Vec<Vec<[f64; 3]>>> {
            let mut ends = vec![];
            let flat = b::free_edge_pts(&self.0, &mut ends).map_err(err)?;
            Ok(Self::split_pts(flat, ends))
        }
        /// Faces alcançadas a partir da face em `p`: 0 nenhuma, 1 ponto, 2 tangente, 3 completo.
        /// Devolve a forma, o número de faces e o de arestas.
        pub fn face_propagate(&self, p: [f64; 3], prop: i32, complement: bool) -> KResult<(Shape, u32, u32)> {
            let mut c = vec![];
            let s = b::face_propagate(&self.0, p[0], p[1], p[2], prop, complement, &mut c).map(Shape).map_err(err)?;
            Ok((s, c[0], c[1]))
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
        /// Forma em binário, para outro processo (sem malha).
        pub fn para_bin(&self) -> KResult<Vec<u8>> {
            b::forma_para_bin(&self.0).map_err(err)
        }
        pub fn de_bin(dados: &[u8]) -> KResult<Shape> {
            b::forma_de_bin(dados).map(Shape).map_err(err)
        }
        /// A mesma topologia (`clone` divide a forma; isto diz se duas são a mesma).
        pub fn mesma(&self, outra: &Shape) -> bool {
            b::forma_mesma(&self.0, &outra.0)
        }
        /// Projeção com linhas ocultas: (polilinha 2D, tipo) — 0 visível, 1 tangente, 2 contorno, 3 oculta.
        pub fn hlr(&self, toward_viewer: [f64; 3], x: [f64; 3], hidden: bool) -> KResult<Vec<(Vec<[f64; 2]>, u8)>> {
            let (mut ends, mut kinds) = (vec![], vec![]);
            let _uma = trava_hlr();
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
        /// Varre esta seção ao longo de uma cadeia plana de pontos (retas e curvas tangentes) com a
        /// normal da chapa como binormal fixa.
        pub fn sm_chain_sweep(&self, pts: &[[f64; 3]], normal: [f64; 3]) -> KResult<Shape> {
            let flat: Vec<f64> = pts.iter().flatten().copied().collect();
            b::sm_chain_sweep(&self.0, &flat, normal[0], normal[1], normal[2]).map(Shape).map_err(err)
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
            self.mesh_angle(deflection, ANGULO_PADRAO)
        }
        /// Apaga a malha guardada na forma. `mesh_angle` aceita a malha pronta com até 10% de
        /// diferença no desvio, então a troca de qualidade chama isto antes; a edição comum não chama.
        pub fn discard_mesh(&self) -> KResult<()> {
            b::discard_mesh(&self.0).map_err(err)
        }
        /// Malha com desvio linear (mm) e ângulo máximo entre normais vizinhas (radianos). Faces com
        /// malha pronta de mesmo ângulo e desvio a até 10% do pedido são reaproveitadas.
        pub fn mesh_angle(&self, deflection: f64, angle: f64) -> KResult<Mesh> {
            b::tessellate(&self.0, deflection, angle).map(malha_do_kernel).map_err(err)
        }
        /// Malha só para a tela (B346), mais fina que a de trabalho: mesma ordem de faces e arestas, sem
        /// mexer na malha guardada na forma. `curv_tol` > 0 divide as arestas internas onde a curvatura
        /// interpolada erra mais que essa fração; `k_ref` é a curvatura (1/mm) abaixo da qual a
        /// diferença deixa de contar.
        pub fn mesh_view(&self, deflection: f64, angle: f64, curv_tol: f64, k_ref: f64) -> KResult<Mesh> {
            b::tessellate_view(&self.0, deflection, angle, curv_tol, k_ref).map(malha_do_kernel).map_err(err)
        }
        /// Índice (na ordem das faces da malha) da face mais próxima do ponto, se estiver a no máximo `limit`.
        pub fn face_index(&self, p: [f64; 3], limit: f64) -> KResult<Option<usize>> {
            let i = b::face_index(&self.0, p[0], p[1], p[2], limit).map_err(err)?;
            Ok((i >= 0).then_some(i as usize))
        }
        /// Arestas (índices de `topology`) que passam a até `limit` do ponto, medindo na curva exata.
        /// Sem costuras nem arestas degeneradas (B391: um ponto qualquer da aresta a identifica).
        pub fn edges_through(&self, p: [f64; 3], limit: f64) -> KResult<Vec<usize>> {
            let v = b::edges_through(&self.0, p[0], p[1], p[2], limit).map_err(err)?;
            Ok(v.into_iter().map(|i| i as usize).collect())
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
        /// Inércia do volume (densidade 1) no centro de massa; `None` sem sólido.
        pub fn inertia(&self) -> Option<InertiaProps> {
            let v = b::inertia_props(&self.0);
            if v.len() < 18 {
                return None;
            }
            Some(InertiaProps {
                matrix: [v[0], v[1], v[2], v[3], v[4], v[5]],
                principal: [v[6], v[7], v[8]],
                axes: [[v[9], v[10], v[11]], [v[12], v[13], v[14]], [v[15], v[16], v[17]]],
            })
        }
        pub fn face_count(&self) -> u32 {
            b::count_faces(&self.0)
        }
        pub fn export_step(&self, path: &str) -> KResult<()> {
            b::export_step(&self.0, path).map_err(err)
        }
        /// STL binário com desvio linear (mm) e ângulo (radianos). A malha sai de uma cópia da forma:
        /// não depende da malha de exibição nem a altera.
        pub fn export_stl(&self, path: &str, deflection: f64, angle: f64) -> KResult<()> {
            b::export_stl(&self.0, path, deflection, angle).map_err(err)
        }
        pub fn import_step(path: &str) -> KResult<Shape> {
            Self::import_step_info(path).map(|(s, _)| s)
        }
        /// Lê o STEP e devolve também o que o arquivo declara. A forma sai em milímetros.
        pub fn import_step_info(path: &str) -> KResult<(Shape, crate::InfoStep)> {
            let mut info = Vec::new();
            let s = b::import_step(path, &mut info).map(Shape).map_err(err)?;
            let mut campos = info.into_iter();
            Ok((s, crate::InfoStep { esquema: campos.next().unwrap_or_default(), unidades: campos.next().unwrap_or_default() }))
        }
    }

    /// Limita os fios do malhador. `None` volta ao padrão: metade dos processadores lógicos, no
    /// mínimo 2. Vale a partir da próxima malha, que espera as malhas em andamento terminarem.
    pub fn definir_fios_malhador(fios: Option<usize>) {
        b::set_mesh_threads(fios.map_or(0, |n| n.clamp(1, i32::MAX as usize) as i32));
    }

    /// Liga o cancelamento a esta thread: as booleanas e a malha em andamento abandonam o cálculo
    /// (erro "cancelado") quando a bandeira sobe. Solta ao largar a guarda.
    pub fn ligar_cancelamento(bandeira: std::sync::Arc<std::sync::atomic::AtomicBool>) -> GuardaCancelamento {
        b::set_cancel_flag(std::sync::Arc::as_ptr(&bandeira) as usize);
        GuardaCancelamento { _bandeira: bandeira }
    }

    /// Mantém a bandeira viva enquanto a thread a usa; ao cair, a thread deixa de ser cancelável.
    pub struct GuardaCancelamento {
        _bandeira: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl Drop for GuardaCancelamento {
        fn drop(&mut self) {
            b::set_cancel_flag(0);
        }
    }

    /// Limita o tempo das booleanas e da malha desta thread: passado o prazo, elas param com erro
    /// "limite de tempo". Vale até largar a guarda. Varredura e filete não têm como ser interrompidos.
    pub fn limitar_tempo(segundos: f64) -> GuardaPrazo {
        b::set_deadline(segundos.max(0.));
        GuardaPrazo { _so_nesta_thread: std::marker::PhantomData }
    }

    /// Ao cair, a thread deixa de ter prazo. Não atravessa threads: o prazo é da thread que o pediu.
    pub struct GuardaPrazo {
        _so_nesta_thread: std::marker::PhantomData<*const ()>,
    }

    impl Drop for GuardaPrazo {
        fn drop(&mut self) {
            b::set_deadline(-1.);
        }
    }

    /// Escolhe o algoritmo do malhador: `true` usa o Delabella. Existe só para medir (B235); o padrão
    /// do programa continua o do OpenCASCADE. Quem troca deve chamar `discard_mesh` antes de comparar.
    pub fn definir_malhador_delabella(ligar: bool) {
        b::set_mesh_algo(ligar as i32);
    }

    pub fn malhador_delabella() -> bool {
        b::mesh_algo() == 1
    }

    /// Teto de fios do malhador em vigor, já limitado aos processadores da máquina.
    pub fn fios_malhador() -> usize {
        b::mesh_threads() as usize
    }

    /// Fios que o malhador usou na última malha.
    pub fn fios_aplicados() -> usize {
        b::mesh_pool_threads() as usize
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

    pub fn hlr_varias(vistas: &[VistaHlr], _: Option<f64>) -> Vec<KResult<Vec<(Vec<[f64; 2]>, u8)>>> {
        vistas.iter().map(|_| no()).collect()
    }

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
        pub fn edges_through(&self, _: [f64; 3], _: f64) -> KResult<Vec<usize>> {
            no()
        }
        pub fn boolean(&self, _: &Shape, _: Bool) -> KResult<Shape> {
            no()
        }
        pub fn comum_medida(&self, _: &Shape) -> KResult<Shape> {
            no()
        }
        pub fn comuns<R: Send>(pares: &[(&Shape, &Shape)], _: usize, f: impl Fn(usize, KResult<Shape>) -> R + Sync) -> Vec<R> {
            (0..pares.len()).map(|i| f(i, no())).collect()
        }
        pub fn em_threads<R: Send>(formas: &[&Shape], _: usize, f: impl Fn(usize, Shape) -> R + Sync) -> Vec<R> {
            formas.iter().enumerate().map(|(i, s)| f(i, (*s).clone())).collect()
        }
        pub fn hole_tool_ext(_: [f64; 3], _: [f64; 3], _: f64, _: f64, _: HoleShape, _: f64, _: f64) -> KResult<Shape> {
            no()
        }
        pub fn ray_hit(&self, _: [f64; 3], _: [f64; 3]) -> KResult<Option<([f64; 3], f64)>> {
            no()
        }
        pub fn ray_hits(&self, _: [f64; 3], _: [f64; 3]) -> KResult<Vec<f64>> {
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
        pub fn sm_chain_sweep(&self, _: &[[f64; 3]], _: [f64; 3]) -> KResult<Shape> {
            no()
        }
        pub fn loft(&self, _: bool, _: bool, _: bool) -> KResult<Shape> {
            no()
        }
        pub fn primitive(_: u8, _: &Frame, _: &[f64]) -> KResult<Shape> {
            no()
        }
        pub fn shell_faces_tagged(&self, _: &[[f64; 3]], _: f64, _: bool, _: &[([f64; 3], f64)], _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn shell_tagged(&self, _: &[[f64; 3]], _: f64, _: bool, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn draft_tagged(&self, _: &[[f64; 3]], _: [f64; 3], _: f64, _: &Frame, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn is_valid(&self) -> bool {
            false
        }
        #[allow(clippy::too_many_arguments)]
        pub fn fillet_variable_tagged(&self, _: f64, _: f64, _: &[[f64; 2]], _: bool, _: &[[f64; 3]], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn defeature_tagged(&self, _: &[[f64; 3]], _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn face_offset_tagged(&self, _: &[[f64; 3]], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
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
        pub fn edges_mode_tagged(&self, _: u8, _: f64, _: f64, _: &[[f64; 3]], _: &[bool], _: f64, _: &[i32]) -> KResult<(Shape, Vec<i32>)> {
            no()
        }
        pub fn cylinder_at(&self, _: [f64; 3]) -> KResult<Cylinder> {
            no()
        }
        pub fn thread_tool_dir(_: [f64; 3], _: [f64; 3], _: [f64; 3], _: bool, _: f64, _: f64, _: f64, _: f64) -> KResult<Shape> {
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
        pub fn fill_surface_x(_: &[Vec<[f64; 3]>], _: &[i32], _: &[f64], _: &Shape, _: &[[f64; 3]]) -> KResult<(Shape, f64, f64, f64)> {
            no()
        }
        pub fn bezier_faces(_: &[f64]) -> KResult<Shape> {
            no()
        }
        pub fn curve_shape(_: &[[f64; 3]]) -> KResult<Shape> {
            no()
        }
        pub fn sew_report(&self) -> KResult<SewReport> {
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
        pub fn grid_surface(_: &[Vec<[f64; 3]>]) -> KResult<Shape> {
            no()
        }
        pub fn connect_check(&self, _: &Shape, _: u32, _: f64) -> KResult<Vec<AmostraConexao>> {
            no()
        }
        pub fn signed_distances(&self, _: &[[f64; 3]]) -> KResult<Vec<f64>> {
            no()
        }
        pub fn iso_curves(&self, _: u32) -> KResult<Vec<Vec<[f64; 3]>>> {
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
        pub fn regions(&self, _: &Shape, _: bool) -> KResult<Vec<Shape>> {
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
        pub fn clip_surface(&self, _: &Shape, _: [f64; 3], _: bool) -> KResult<Shape> {
            no()
        }
        pub fn edge_plane(&self, _: [f64; 3], _: [f64; 3], _: [f64; 3]) -> KResult<Vec<[f64; 3]>> {
            no()
        }
        pub fn edge_at(&self, _: [f64; 3]) -> KResult<Vec<[f64; 3]>> {
            no()
        }
        pub fn free_edges(&self) -> KResult<Vec<Vec<[f64; 3]>>> {
            no()
        }
        pub fn face_propagate(&self, _: [f64; 3], _: i32, _: bool) -> KResult<(Shape, u32, u32)> {
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
        pub fn para_bin(&self) -> KResult<Vec<u8>> {
            no()
        }
        pub fn de_bin(_: &[u8]) -> KResult<Shape> {
            no()
        }
        pub fn mesma(&self, _: &Shape) -> bool {
            false
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
        pub fn discard_mesh(&self) -> KResult<()> {
            no()
        }
        pub fn mesh_angle(&self, _: f64, _: f64) -> KResult<Mesh> {
            no()
        }
        pub fn mesh_view(&self, _: f64, _: f64, _: f64, _: f64) -> KResult<Mesh> {
            no()
        }
        pub fn bbox(&self) -> Option<([f64; 3], [f64; 3])> {
            None
        }
        pub fn mass(&self) -> MassProps {
            MassProps::default()
        }
        pub fn inertia(&self) -> Option<InertiaProps> {
            None
        }
        pub fn face_count(&self) -> u32 {
            0
        }
        pub fn export_step(&self, _: &str) -> KResult<()> {
            no()
        }
        pub fn export_stl(&self, _: &str, _: f64, _: f64) -> KResult<()> {
            no()
        }
        pub fn import_step(_: &str) -> KResult<Shape> {
            no()
        }
        pub fn import_step_info(_: &str) -> KResult<(Shape, crate::InfoStep)> {
            no()
        }
    }

    pub fn version() -> String {
        "sem kernel".into()
    }

    pub fn definir_fios_malhador(_: Option<usize>) {}

    pub fn ligar_cancelamento(_: std::sync::Arc<std::sync::atomic::AtomicBool>) -> GuardaCancelamento {
        GuardaCancelamento
    }

    pub struct GuardaCancelamento;

    pub fn limitar_tempo(_: f64) -> GuardaPrazo {
        GuardaPrazo
    }

    pub struct GuardaPrazo;

    pub fn definir_malhador_delabella(_: bool) {}

    pub fn malhador_delabella() -> bool {
        false
    }

    pub fn fios_malhador() -> usize {
        1
    }

    pub fn fios_aplicados() -> usize {
        1
    }
}

/// Uma vista para `hlr_varias`: forma, direção para o observador, eixo x da folha e se traça as
/// linhas ocultas.
pub struct VistaHlr<'a> {
    pub forma: &'a Shape,
    pub para: [f64; 3],
    pub x: [f64; 3],
    pub ocultas: bool,
    /// roda as threads com prioridade abaixo do normal (fora da thread da interface)
    pub prioridade_baixa: bool,
}

pub use imp::{hlr_varias, limitar_tempo, GuardaPrazo, ligar_cancelamento, GuardaCancelamento, definir_fios_malhador, definir_malhador_delabella, malhador_delabella, fios_aplicados, fios_malhador, version, Shape, AVAILABLE};

/// Põe a thread atual abaixo da prioridade normal, para o cálculo não disputar processador com a
/// interface. Chame uma vez, dentro da thread de cálculo. Devolve se a prioridade mudou.
#[cfg(windows)]
pub fn reduzir_prioridade_da_thread() -> bool {
    use std::ffi::c_void;
    const THREAD_PRIORITY_BELOW_NORMAL: i32 = -1;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
    }
    // SAFETY: GetCurrentThread devolve um pseudo-identificador sempre válido na própria thread.
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) != 0 }
}

/// Fora do Windows a prioridade fica como está.
#[cfg(not(windows))]
pub fn reduzir_prioridade_da_thread() -> bool {
    false
}

#[cfg(all(test, occt))]
mod tests {
    use super::*;

    const XY: Frame = Frame { origin: [0.; 3], x: [1., 0., 0.], normal: [0., 0., 1.] };

    fn rect(w: f64, h: f64) -> Loop2 {
        let (a, b, c, d) = ([-w / 2., -h / 2.], [w / 2., -h / 2.], [w / 2., h / 2.], [-w / 2., h / 2.]);
        vec![Seg2::Line(a, b), Seg2::Line(b, c), Seg2::Line(c, d), Seg2::Line(d, a)]
    }

    #[test]
    fn normais_de_exibicao_vem_da_superficie() {
        let s = Shape::cylinder_surface([0., 0., 0.], [0., 0., 1.], 10., 0., 30.).unwrap();
        let m = s.mesh_angle(0.5, 0.5).unwrap();
        assert!(!m.indices.is_empty());
        let at = |v: &[f32], i: u32| [v[i as usize * 3], v[i as usize * 3 + 1], v[i as usize * 3 + 2]];
        for i in 0..(m.positions.len() / 3) as u32 {
            let (p, n) = (at(&m.positions, i), at(&m.normals, i));
            let radial = (p[0] * n[0] + p[1] * n[1]) / 10.;
            assert!((radial.abs() - 1.).abs() < 1e-4 && n[2].abs() < 1e-4, "vértice {i}: {p:?} {n:?}");
        }
        // o sentido acompanha a orientação dos triângulos
        for t in m.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| at(&m.positions, i));
            let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let g = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let n = at(&m.normals, t[0]);
            assert!(g[0] * n[0] + g[1] * n[1] + g[2] * n[2] > 0.);
        }
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

    /// Pasta nova com acento e espaço, só deste teste.
    fn pasta(teste: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("Fenkra açaí {} {teste}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn triangulos_stl(p: &std::path::Path) -> usize {
        let b = std::fs::read(p).unwrap();
        let n = u32::from_le_bytes(b[80..84].try_into().unwrap()) as usize;
        assert_eq!(b.len(), 84 + 50 * n, "o tamanho é o de um STL binário");
        n
    }

    #[test]
    fn step_grava_e_le_com_acento_e_espaco() {
        let dir = pasta("step");
        let alvo = dir.join("peça com espaço.step");
        let caixa = Shape::face(&XY, &[rect(40., 30.)]).unwrap().extrude([0., 0., 1.], 20., 0.).unwrap();
        caixa.export_step(alvo.to_str().unwrap()).unwrap();
        // o OCCT espera UTF-8: em outra codificação o arquivo sairia com outro nome
        assert!(alvo.is_file());
        let lida = Shape::import_step(alvo.to_str().unwrap()).unwrap();
        assert!((lida.mass().volume - 24000.).abs() < 1e-3);
        assert_eq!(lida.face_count(), 6);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stl_e_binario_e_nao_depende_da_malha_da_tela() {
        let dir = pasta("stl");
        let alvo = dir.join("peça de impressão.stl");
        let cil = Shape::face(&XY, &[vec![Seg2::Circle([0., 0.], 20.)]]).unwrap().extrude([0., 0., 1.], 30., 0.).unwrap();
        let caminho = alvo.to_str().unwrap();

        cil.export_stl(caminho, 0.5, 0.6).unwrap();
        let limpa = triangulos_stl(&alvo);
        // a tela malha mais fino que o arquivo pedido: o BRepMesh aceitaria essa malha pronta
        let tela = cil.mesh_angle(0.005, 0.2).unwrap();
        cil.export_stl(caminho, 0.5, 0.6).unwrap();
        assert_eq!(triangulos_stl(&alvo), limpa);

        // e a exportação não mexe no que a tela já tem
        assert_eq!(cil.mesh_angle(0.005, 0.2).unwrap().indices, tela.indices);
        cil.export_stl(caminho, 0.005, 0.2).unwrap();
        assert!(triangulos_stl(&alvo) > limpa);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn exportar_para_pasta_inexistente_devolve_erro() {
        let raiz = pasta("sem pasta");
        let dir = raiz.join("não existe");
        let caixa = Shape::face(&XY, &[rect(10., 10.)]).unwrap().extrude([0., 0., 1.], 5., 0.).unwrap();
        assert!(caixa.export_step(dir.join("a.step").to_str().unwrap()).is_err());
        assert!(caixa.export_stl(dir.join("a.stl").to_str().unwrap(), 0.1, 0.5).is_err());
        assert!(Shape::import_step(dir.join("a.step").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(raiz).unwrap();
    }

    #[test]
    fn malha_refaz_com_o_desvio_e_o_angulo_pedidos() {
        let cil = Shape::face(&XY, &[vec![Seg2::Circle([0., 0.], 20.)]]).unwrap().extrude([0., 0., 1.], 30., 0.).unwrap();
        let fina = cil.mesh_angle(0.005, 0.2).unwrap().indices.len();
        // a mesma forma, agora com pedido grosso: a malha anterior não sobra
        let grossa = cil.mesh_angle(0.5, 0.6).unwrap().indices.len();
        assert!(fina > grossa, "{fina} vs {grossa}");
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

    /// placa com grade de furos filetados, o pior caso do malhador
    fn placa(n: usize) -> Shape {
        let w = n as f64 * 20.;
        let mut l = vec![rect(w, w)];
        for i in 0..n {
            for j in 0..n {
                l.push(vec![Seg2::Circle([-w / 2. + 20. * (i as f64 + 0.5), -w / 2. + 20. * (j as f64 + 0.5)], 5.)]);
            }
        }
        Shape::face(&XY, &l).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap().fillet_all(1.).unwrap()
    }

    /// arestas de triângulo, soldadas por posição, que não têm par: cada uma é borda de um furo na malha
    fn bordas_abertas(m: &Mesh) -> usize {
        let q = |i: u32| {
            let k = i as usize * 3;
            [0, 1, 2].map(|c| (m.positions[k + c] * 2000.).round() as i32)
        };
        let mut n = std::collections::HashMap::new();
        for t in m.indices.chunks_exact(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let (a, b) = (q(a), q(b));
                if a != b {
                    *n.entry(if a < b { (a, b) } else { (b, a) }).or_insert(0) += 1;
                }
            }
        }
        n.values().filter(|c| **c % 2 != 0).count()
    }

    /// B393: várias vistas de uma vez. A exata dá o mesmo que `hlr` vista a vista (corre uma por
    /// vez); a por malha, em threads, traça todas as vistas e a forma volta inteira pelo binário.
    #[test]
    fn varias_vistas_exatas_e_por_malha() {
        let s = placa(3);
        let entalhe = Shape::box_tool([27., -5., 3.], [1., 0., 0.], [0., 0., 1.], [6., 10., 4.]).unwrap();
        let e = s.boolean(&entalhe, Bool::Cut).unwrap();
        let dirs = [([0., 0., 1.], [1., 0., 0.]), ([0., -1., 0.], [1., 0., 0.]), ([1., 0., 0.], [0., 1., 0.])];
        let vistas: Vec<_> = dirs.iter().map(|(p, x)| VistaHlr { forma: &e, para: *p, x: *x, ocultas: true, prioridade_baixa: false }).collect();
        let exatas = hlr_varias(&vistas, None);
        for ((p, x), r) in dirs.iter().zip(&exatas) {
            let uma = e.hlr(*p, *x, true).unwrap();
            assert_eq!(r.as_ref().unwrap().len(), uma.len());
        }
        let malha = hlr_varias(&vistas, Some(0.05));
        assert!(malha.iter().all(|r| r.as_ref().is_ok_and(|l| l.iter().any(|(_, k)| *k == 0))));
        let volta = Shape::de_bin(&e.para_bin().unwrap()).unwrap();
        assert!((volta.mass().volume - e.mass().volume).abs() < 1e-6);
        assert!(volta.mesma(&volta) && !volta.mesma(&e));
        assert!(e.clone().mesma(&e), "clone divide a topologia");
    }

    #[test]
    fn edicao_reaproveita_faces_intactas_sem_abrir_a_malha() {
        let s = placa(3);
        let antes = s.mesh_angle(0.3, 0.35).unwrap();
        assert_eq!(bordas_abertas(&antes), 0);
        // entalhe numa lateral: as faces de cima e de baixo não mudam e vêm com a malha pronta
        let entalhe = Shape::box_tool([27., -5., 3.], [1., 0., 0.], [0., 0., 1.], [6., 10., 4.]).unwrap();
        let e = s.boolean(&entalhe, Bool::Cut).unwrap();
        let m = e.mesh_angle(0.3, 0.35).unwrap();
        assert_eq!(bordas_abertas(&m), 0);
        assert_eq!(m.face_ranges.len(), antes.face_ranges.len() + 5);
        assert!((s.mass().volume - e.mass().volume - 3. * 10. * 4.).abs() < 1e-6);
        let (a, b) = (s.bbox().unwrap(), e.bbox().unwrap());
        assert!(a.0.iter().chain(&a.1).zip(b.0.iter().chain(&b.1)).all(|(x, y)| (x - y).abs() < 1e-3));
        // a malha reaproveitada é a mesma de uma forma que nunca foi malhada
        let limpa = placa(3).boolean(&entalhe, Bool::Cut).unwrap();
        assert_eq!(limpa.mesh_angle(0.3, 0.35).unwrap().indices.len(), m.indices.len());
    }

    #[test]
    fn malha_pronta_vale_com_folga_de_dez_por_cento_no_desvio() {
        let s = Shape::cylinder_surface([0., 0., 0.], [0., 0., 1.], 10., 0., 30.).unwrap();
        let tris = |d, a| s.mesh_angle(d, a).unwrap().indices.len() / 3;
        let base = tris(0.02, 0.6);
        let perto = Shape::cylinder_surface([0., 0., 0.], [0., 0., 1.], 10., 0., 30.).unwrap().mesh_angle(0.0185, 0.6).unwrap().indices.len() / 3;
        assert!(perto != base, "o caso precisa distinguir as duas malhas: {perto}");
        assert_eq!(tris(0.0185, 0.6), base, "desvio a menos de 10% reaproveita, como na caixa que muda um pouco");
        s.discard_mesh().unwrap();
        assert_eq!(tris(0.0185, 0.6), perto, "descartar faz valer o pedido exato");
        // o malhador não confere o ângulo: quem confere é a fachada, nos dois sentidos
        let fino = tris(0.0185, 0.1);
        assert!(fino > perto, "ângulo menor refaz: {perto} -> {fino}");
        assert_eq!(tris(0.0185, 0.6), perto, "ângulo maior refaz");
    }

    #[test]
    fn suavidade_das_arestas_nao_depende_da_posicao() {
        let s = placa(2);
        let a = s.mesh_angle(0.3, 0.35).unwrap();
        // filete: tangente às duas faces; sobram vivas só as arestas sem filete (nenhuma aqui)
        assert!(a.edge_smooth.iter().any(|x| *x != 0));
        let lado = Shape::face(&XY, &[rect(20., 20.)]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        assert!(lado.mesh(0.3).unwrap().edge_smooth.iter().all(|x| *x == 0), "cubo só tem aresta viva");
        let movida = s.transformed(Xform::Translate([100., 50., 7.])).unwrap();
        assert_eq!(movida.mesh_angle(0.3, 0.35).unwrap().edge_smooth, a.edge_smooth);
        let girada = s.transformed(Xform::Rotate([0.; 3], [1., 0., 0.], 33.)).unwrap();
        assert_eq!(girada.mesh_angle(0.3, 0.35).unwrap().edge_smooth, a.edge_smooth);
    }

    #[test]
    fn ponto_da_assinatura_cai_na_face_do_furo() {
        // centroide no eixo do furo: a projeção falha e o ponto sai da grade UV
        let s = Shape::face(&XY, &[rect(40., 40.), vec![Seg2::Circle([3., 4.], 5.)]]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let m = s.mesh(0.2).unwrap();
        let furo = m.face_info.iter().find(|f| f.kind == 1).unwrap();
        let [x, y, z] = furo.point;
        assert!((((x - 3.).powi(2) + (y - 4.).powi(2)).sqrt() - 5.).abs() < 1e-6, "{:?}", furo.point);
        assert!((z - 5.).abs() < 1.1, "perto do centroide: {z}");
    }

    #[test]
    fn fios_do_malhador_limitam_sem_mudar_a_malha() {
        let padrao = fios_malhador();
        assert!(padrao >= 1);
        definir_fios_malhador(Some(1));
        assert_eq!(fios_malhador(), 1);
        let um = placa(3).mesh_angle(0.3, 0.35).unwrap();
        assert_eq!(fios_aplicados(), 1, "o limite chega ao pool do OpenCASCADE");
        definir_fios_malhador(Some(usize::MAX));
        let todos = fios_malhador();
        assert!(todos >= padrao, "limitado aos processadores da máquina: {todos}");
        let muitos = placa(3).mesh_angle(0.3, 0.35).unwrap();
        assert_eq!(fios_aplicados(), todos);
        definir_fios_malhador(None);
        assert_eq!(fios_malhador(), padrao);
        assert!(padrao == todos || padrao >= 2 && padrao <= todos / 2 + 1, "padrão {padrao} de {todos}");
        assert_eq!(um.indices, muitos.indices);
        assert_eq!(um.positions, muitos.positions);
    }

    /// Malha mais grossa que a configuração aceita: 45° e flecha grande.
    const GROSSA: (f64, f64) = (2.0, 0.785);

    fn ponto(m: &Mesh, i: usize) -> [f64; 3] {
        [0, 1, 2].map(|k| m.positions[i * 3 + k] as f64)
    }
    fn normal(m: &Mesh, i: usize) -> [f64; 3] {
        [0, 1, 2].map(|k| m.normals[i * 3 + k] as f64)
    }

    #[test]
    fn esfera_tem_normal_e_curvatura_exatas_em_todo_no_inclusive_nos_polos() {
        let r = 40.;
        let m = Shape::sphere_surface([0., 0., 0.], r).unwrap().mesh_angle(GROSSA.0, GROSSA.1).unwrap();
        let n = m.positions.len() / 3;
        assert!(n > 10);
        let polos = (0..n).filter(|i| ponto(&m, *i)[2].abs() > r - 1e-3).count();
        assert!(polos > 0, "a malha chega aos polos");
        for i in 0..n {
            let (p, q) = (ponto(&m, i), normal(&m, i));
            let erro = (0..3).map(|k| (p[k] / r - q[k]).powi(2)).sum::<f64>().sqrt();
            assert!(erro < 1e-3, "nó {i} em {p:?}: normal {q:?}");
            let (k1, k2) = (m.curvature[i * 2].abs(), m.curvature[i * 2 + 1].abs());
            assert!((k1 - 1. / 40.).abs() < 1e-5 && (k2 - 1. / 40.).abs() < 1e-5, "nó {i} em {p:?}: {k1} {k2}");
        }
    }

    #[test]
    fn cilindro_tem_curvatura_exata() {
        let m = Shape::cylinder_surface([0., 0., 0.], [0., 0., 1.], 30., 0., 80.).unwrap().mesh_angle(GROSSA.0, GROSSA.1).unwrap();
        for c in m.curvature.chunks_exact(2) {
            let (a, b) = (c[0].abs().max(c[1].abs()), c[0].abs().min(c[1].abs()));
            assert!((a - 1. / 30.).abs() < 1e-6 && b < 1e-6, "{c:?}");
        }
    }

    /// Curvatura exata do toro no ponto: 1/r na seção e cos(t)/(R + r cos(t)) na volta.
    fn curvatura_do_toro(p: [f64; 3], maior: f64, menor: f64) -> (f64, f64) {
        let cos = ((p[0] * p[0] + p[1] * p[1]).sqrt() - maior) / menor;
        (1. / menor, cos / (maior + menor * cos))
    }

    fn toro() -> Shape {
        let f = Frame { origin: [0., 0., 0.], x: [1., 0., 0.], normal: [0., 0., 1.] };
        Shape::primitive(4, &f, &[40., 15.]).unwrap()
    }

    /// Erro médio, por área, da curvatura interpolada no centro dos triângulos, contra a do toro. A
    /// interpolação é a que a tela faz; o centro do triângulo é onde ela mais se afasta dos nós. É a
    /// média porque a borda da face não se divide: os triângulos encostados nela ficam como estão.
    fn erro_no_meio_dos_triangulos(m: &Mesh) -> f64 {
        let (mut soma, mut total) = (0f64, 0f64);
        for t in m.indices.chunks_exact(3) {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            let lado = |x: usize, y: usize| [0, 1, 2].map(|k| ponto(m, y)[k] - ponto(m, x)[k]);
            let (u, v) = (lado(a, b), lado(a, c));
            let area = ((u[1] * v[2] - u[2] * v[1]).powi(2) + (u[2] * v[0] - u[0] * v[2]).powi(2) + (u[0] * v[1] - u[1] * v[0]).powi(2)).sqrt() / 2.;
            let p: [f64; 3] = [0, 1, 2].map(|k| (ponto(m, a)[k] + ponto(m, b)[k] + ponto(m, c)[k]) / 3.);
            // a curvatura da volta é a menor em módulo: a da seção vale 1/15 em todo o toro
            let da_volta = |i: usize| {
                let (x, y) = (m.curvature[i * 2] as f64, m.curvature[i * 2 + 1] as f64);
                if x.abs() < y.abs() { x } else { y }
            };
            let interpolada = (da_volta(a) + da_volta(b) + da_volta(c)) / 3.;
            // o centro do triângulo fica um pouco dentro do toro: leva o ponto até a superfície
            let rho = (p[0] * p[0] + p[1] * p[1]).sqrt();
            let (dx, dz) = (rho - 40., p[2]);
            let l = (dx * dx + dz * dz).sqrt().max(1e-9);
            let na_superficie = [(40. + dx / l * 15.) * p[0] / rho, (40. + dx / l * 15.) * p[1] / rho, dz / l * 15.];
            let (_, k2) = curvatura_do_toro(na_superficie, 40., 15.);
            soma += (interpolada.abs() - k2.abs()).abs() * area;
            total += area;
        }
        soma / total
    }

    #[test]
    fn curvatura_refinada_acompanha_a_superficie_entre_os_nos() {
        let (d, a) = (0.5, GROSSA.1);
        let comum = toro().mesh_view(d, a, 0., 0.).unwrap();
        let fina = toro().mesh_view(d, a, 0.005, 1e-3).unwrap();
        assert!(fina.indices.len() > comum.indices.len(), "{} vs {}", fina.indices.len(), comum.indices.len());
        assert_eq!(fina.face_ranges.len(), comum.face_ranges.len());
        let (e0, e1) = (erro_no_meio_dos_triangulos(&comum), erro_no_meio_dos_triangulos(&fina));
        assert!(e1 < e0 * 0.6, "erro {e0} virou {e1}");
        // o nó novo fica na superfície exata e com a curvatura de lá
        for i in 0..fina.positions.len() / 3 {
            let (k1, k2) = curvatura_do_toro(ponto(&fina, i), 40., 15.);
            let (a, b) = (fina.curvature[i * 2] as f64, fina.curvature[i * 2 + 1] as f64);
            assert!((a.abs().max(b.abs()) - k1.abs().max(k2.abs())).abs() < 1e-4, "nó {i}: {a} {b} contra {k1} {k2}");
        }
    }

    #[test]
    fn malha_de_tela_nao_mexe_na_malha_de_trabalho() {
        let s = toro();
        let trabalho = s.mesh_angle(1., 0.5).unwrap();
        let tela = s.mesh_view(0.02, 0.087, 0., 0.).unwrap();
        assert!(tela.indices.len() > trabalho.indices.len() * 4, "{} vs {}", tela.indices.len(), trabalho.indices.len());
        assert_eq!(tela.face_ranges.len(), trabalho.face_ranges.len());
        assert_eq!(tela.edge_ranges.len(), trabalho.edge_ranges.len());
        assert_eq!(tela.edge_smooth, trabalho.edge_smooth);
        let depois = s.mesh_angle(1., 0.5).unwrap();
        assert_eq!(depois.indices, trabalho.indices);
        assert_eq!(depois.positions, trabalho.positions);
        assert!((s.mass().volume - 2. * std::f64::consts::PI.powi(2) * 40. * 15f64.powi(2)).abs() < 1.);
    }

    #[test]
    fn furo_no_plano_acompanha_a_malha_de_tela_sem_fresta() {
        // a borda do furo é da aresta: a tampa plana e a parede do furo recebem os mesmos pontos
        let s = Shape::face(&XY, &[rect(60., 60.), vec![Seg2::Circle([0., 0.], 12.)]]).unwrap().extrude([0., 0., 1.], 10., 0.).unwrap();
        let m = s.mesh_view(0.01, 0.087, 0.02, 1e-3).unwrap();
        let mut bordas = std::collections::HashMap::<[[u32; 3]; 2], i32>::new();
        let chave = |i: u32| [0, 1, 2].map(|k| m.positions[i as usize * 3 + k].to_bits());
        for t in m.indices.chunks_exact(3) {
            for e in 0..3 {
                let (a, b) = (chave(t[e]), chave(t[(e + 1) % 3]));
                *bordas.entry(if a < b { [a, b] } else { [b, a] }).or_default() += 1;
            }
        }
        assert!(bordas.values().all(|n| *n == 2), "a casca da malha fecha: nenhuma aresta sobra");
    }

    #[test]
    fn comuns_em_threads_dao_o_mesmo_volume_que_um_por_vez() {
        // B394: a mesma placa entra em todos os pares, como o aro do kart; um bloco fica inteiro
        // dentro dela (o comum devolve as faces da entrada) e cada resultado é tesselado na thread
        let furos: Vec<Seg2> = (0..6).map(|i| Seg2::Circle([-40. + 16. * i as f64, 0.], 4.)).collect();
        let mut lacos = vec![rect(120., 50.)];
        lacos.extend(furos.into_iter().map(|c| vec![c]));
        let placa = Shape::face(&XY, &lacos).unwrap().extrude([0., 0., 1.], 20., 0.).unwrap();
        let mut outros = vec![];
        for i in 0..12 {
            let x = -55. + 10. * i as f64;
            let cil = Shape::face(&Frame { origin: [x, 12., -5.], ..XY }, &[vec![Seg2::Circle([0., 0.], 3. + i as f64 * 0.4)]]).unwrap();
            outros.push(cil.extrude([0., 0., 1.], 15. + i as f64, 0.).unwrap());
            outros.push(Shape::box_tool([x, -30., 2. + i as f64], [1., 0.3, 0.], [0., 0., 1.], [7., 20., 5.]).unwrap());
        }
        outros.push(Shape::box_tool([10., 10., 5.], [1., 0., 0.], [0., 0., 1.], [3., 3., 3.]).unwrap());
        let mut pares: Vec<(&Shape, &Shape)> = outros.iter().map(|o| (&placa, o)).collect();
        pares.extend(outros.windows(2).map(|w| (&w[0], &w[1])));
        let medir = |i: usize, r: KResult<Shape>| {
            let c = r.unwrap();
            let v = c.mass().volume;
            // desvio diferente a cada par: a malha guardada nas faces muda de um par para o outro
            let m = c.mesh_angle(0.05 + 0.01 * (i % 7) as f64, 0.3).unwrap();
            (v, m.indices.is_empty())
        };
        let um = Shape::comuns(&pares, 1, medir);
        let esperado: Vec<f64> = pares.iter().map(|(a, b)| a.boolean(b, Bool::Common).unwrap().mass().volume).collect();
        assert!(esperado.iter().filter(|v| **v > 1e-6).count() > 20, "os pares se cruzam de verdade");
        for (i, ((v, vazia), e)) in um.iter().zip(&esperado).enumerate() {
            assert!((v - e).abs() <= 1e-6 * e.max(1.), "par {i}: {v} sem unificar, {e} com");
            assert_eq!(*vazia, *v < 1e-9, "par {i}: malha do comum");
        }
        assert!((um[24].0 - 27.).abs() < 1e-6, "bloco inteiro dentro da placa: {}", um[24].0);
        for _ in 0..4 {
            let varios = Shape::comuns(&pares, 8, medir);
            assert_eq!(varios.len(), pares.len());
            for (i, (a, b)) in um.iter().zip(&varios).enumerate() {
                assert!((a.0 - b.0).abs() <= 1e-9 * a.0.max(1.), "par {i}: {} em threads, {} um por vez", b.0, a.0);
                assert_eq!(a.1, b.1, "par {i}");
            }
        }
        // cada forma tesselada na sua thread, numa cópia: a mesma malha que a original daria
        let refs: Vec<&Shape> = outros.iter().chain([&placa]).collect();
        let malha = |_: usize, s: Shape| {
            let m = s.mesh_angle(0.05, 0.3).unwrap();
            (m.indices.len(), s.mass().volume)
        };
        let sozinho = Shape::em_threads(&refs, 1, malha);
        for _ in 0..3 {
            assert_eq!(Shape::em_threads(&refs, 8, malha), sozinho);
        }
        // as entradas saem como entraram
        assert!((placa.mass().volume - (120. * 50. - 6. * std::f64::consts::PI * 16.) * 20.).abs() < 1e-3);
    }

    #[cfg(windows)]
    #[test]
    fn thread_de_calculo_fica_abaixo_da_normal() {
        assert!(std::thread::spawn(reduzir_prioridade_da_thread).join().unwrap());
    }
}
