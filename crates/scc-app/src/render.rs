//! Viewport 3D própria (wgpu): renderiza em textura offscreen (MSAA + profundidade) que o egui exibe.
//! Só renderiza de novo quando câmera, cena, estilo ou tamanho mudam.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

const SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    params: vec4<f32>,     // x = estilo
    bg_top: vec4<f32>,
    bg_bottom: vec4<f32>,  // w = 1 → degradê
    paper: vec4<f32>,      // cor das faces no estilo "linhas ocultas"
};
@group(0) @binding(0) var<uniform> u: U;

// ---------- fundo
struct BOut { @builtin(position) clip: vec4<f32>, @location(0) y: f32 };
@vertex fn vs_bg(@builtin(vertex_index) i: u32) -> BOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: BOut;
    o.clip = vec4<f32>(p[i], 0.0, 1.0);
    o.y = p[i].y * 0.5 + 0.5;
    return o;
}
@fragment fn fs_bg(i: BOut) -> @location(0) vec4<f32> {
    var c = u.bg_bottom.rgb;
    if (u.bg_bottom.w > 0.5) { c = mix(u.bg_bottom.rgb, u.bg_top.rgb, clamp(i.y, 0.0, 1.0)); }
    return vec4<f32>(c, 1.0);
}

// ---------- faces
struct MIn { @location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @location(2) col: vec4<f32>, @location(3) mat: vec4<f32> };
struct MOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) n: vec3<f32>, @location(1) col: vec4<f32>, @location(2) wp: vec3<f32>, @location(3) mat: vec4<f32>,
};
@vertex fn vs_mesh(v: MIn) -> MOut {
    var o: MOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.n = v.nrm; o.col = v.col; o.wp = v.pos; o.mat = v.mat;
    return o;
}
@fragment fn fs_mesh(i: MOut) -> @location(0) vec4<f32> {
    let mode = u.params.x;
    let v = normalize(u.eye.xyz - i.wp);
    var n = normalize(i.n);
    if (dot(n, v) < 0.0) { n = -n; }
    if (mode > 1.5 && mode < 2.5) { return vec4<f32>(u.paper.rgb, 1.0); }

    let base = i.col.rgb;
    let rough = i.mat.x;
    let metal = i.mat.y;
    var alpha = i.mat.z * i.col.a;

    var c: vec3<f32>;
    if (mode > 4.5) {
        // semi-renderizado: céu/chão + luz principal + reflexo de ambiente + Fresnel
        let sky = vec3<f32>(0.80, 0.85, 0.95);
        let ground = vec3<f32>(0.22, 0.20, 0.18);
        let amb = mix(ground, sky, n.z * 0.5 + 0.5);
        let l = normalize(vec3<f32>(0.45, -0.55, 0.85) + v * 0.6);
        let diff = max(dot(n, l), 0.0);
        let h = normalize(l + v);
        let shin = mix(400.0, 6.0, rough);
        let spec = pow(max(dot(n, h), 0.0), shin) * (1.0 - rough * 0.85);
        let f0 = mix(vec3<f32>(0.04), base, metal);
        let fres = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
        let r = reflect(-v, n);
        let env = mix(ground, sky, smoothstep(-0.3, 0.4, r.z));
        let diffc = base * (1.0 - metal);
        c = diffc * (amb * 0.45 + diff * 0.75) + fres * (env * (1.0 - rough * 0.7) * 0.9 + vec3<f32>(spec) * 1.4);
        if (metal > 0.5) { c = c + base * env * 0.35; }
    } else {
        let d = abs(dot(n, v));
        let k = normalize(vec3<f32>(0.3, 0.5, 1.0));
        let fill = max(dot(n, k), 0.0) * 0.25;
        let spec = pow(d, 48.0) * 0.22 * (1.0 - rough);
        c = base * (0.30 + 0.55 * d + fill) + vec3<f32>(spec);
    }
    if (mode > 3.5 && mode < 4.5) {
        // raio-X
        c = mix(c, vec3<f32>(0.75, 0.85, 1.0), 0.35);
        alpha = 0.22;
    }
    return vec4<f32>(c, alpha);
}

// ---------- linhas
struct LIn { @location(0) pos: vec3<f32>, @location(1) col: vec4<f32> };
struct LOut { @builtin(position) clip: vec4<f32>, @location(0) col: vec4<f32> };
@vertex fn vs_line(v: LIn) -> LOut {
    var o: LOut;
    var p = u.view_proj * vec4<f32>(v.pos, 1.0);
    p.z = p.z - 0.0006 * p.w;   // arestas sempre por cima das faces coincidentes
    o.clip = p; o.col = v.col;
    return o;
}
@fragment fn fs_line(i: LOut) -> @location(0) vec4<f32> { return i.col; }
"#;

/// Estilo de exibição (RF-UI-015).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum DisplayMode {
    #[default]
    SombreadoArestas,
    Sombreado,
    LinhasOcultas,
    Arame,
    RaioX,
    SemiRenderizado,
}

impl DisplayMode {
    pub const ALL: [DisplayMode; 6] = [
        DisplayMode::SombreadoArestas,
        DisplayMode::Sombreado,
        DisplayMode::LinhasOcultas,
        DisplayMode::Arame,
        DisplayMode::RaioX,
        DisplayMode::SemiRenderizado,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::SombreadoArestas => "Sombreado com arestas",
            DisplayMode::Sombreado => "Sombreado (sem arestas)",
            DisplayMode::LinhasOcultas => "Linhas ocultas removidas",
            DisplayMode::Arame => "Arame",
            DisplayMode::RaioX => "Raio-X",
            DisplayMode::SemiRenderizado => "Semi-renderizado",
        }
    }
    fn code(self) -> f32 {
        match self {
            DisplayMode::SombreadoArestas => 0.,
            DisplayMode::Sombreado => 1.,
            DisplayMode::LinhasOcultas => 2.,
            DisplayMode::Arame => 3.,
            DisplayMode::RaioX => 4.,
            DisplayMode::SemiRenderizado => 5.,
        }
    }
}

/// Aparência da viewport derivada das configurações.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ViewStyle {
    pub mode: DisplayMode,
    pub bg_top: [u8; 3],
    pub bg_bottom: [u8; 3],
    pub gradient: bool,
    pub edges_in_render: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub col: [f32; 4],
    /// rugosidade, metalicidade, opacidade, —
    pub mat: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub col: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    params: [f32; 4],
    bg_top: [f32; 4],
    bg_bottom: [f32; 4],
    paper: [f32; 4],
}

/// Cena pronta para a GPU.
#[derive(Default)]
pub struct Scene {
    pub mesh: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// arestas dos corpos (dependem do estilo de exibição)
    pub edges: Vec<LineVertex>,
    /// linhas com teste de profundidade (sketches, planos)
    pub lines: Vec<LineVertex>,
    /// linhas sempre visíveis (seleção, eixos, pontos)
    pub overlay: Vec<LineVertex>,
    pub bounds: Option<(Vec3, Vec3)>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub ortho: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { target: Vec3::ZERO, yaw: -0.785, pitch: 0.6, dist: 300., ortho: false }
    }
}

const FOV: f32 = 0.6;

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        self.target + self.dist * Vec3::new(cp * self.yaw.cos(), cp * self.yaw.sin(), sp)
    }
    fn up(&self) -> Vec3 {
        if self.pitch.abs() > 1.55 {
            Vec3::new(-self.yaw.cos(), -self.yaw.sin(), 0.) * self.pitch.signum()
        } else {
            Vec3::Z
        }
    }
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(self.eye(), self.target, self.up());
        let aspect = aspect.max(0.01);
        // wgpu: Z do NDC em [0, 1], Y para cima
        let proj = if self.ortho {
            let h = self.dist * (FOV / 2.).tan();
            glam::camera::rh::proj::directx::orthographic(-h * aspect, h * aspect, -h, h, -self.dist * 50., self.dist * 50.)
        } else {
            glam::camera::rh::proj::directx::perspective(FOV, aspect, self.dist * 0.01, self.dist * 50.)
        };
        proj * view
    }
    /// Vetores direita/cima no mundo (para pan).
    pub fn basis(&self) -> (Vec3, Vec3) {
        let f = (self.target - self.eye()).normalize();
        let r = f.cross(self.up()).try_normalize().unwrap_or(Vec3::X);
        (r, r.cross(f))
    }
    pub fn fit(&mut self, b: (Vec3, Vec3)) {
        self.target = (b.0 + b.1) / 2.;
        self.dist = ((b.1 - b.0).length() * 1.6).max(20.);
    }
    pub fn set_view(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
    }
    /// Olha de frente para um plano (olho do lado da normal).
    pub fn look_along(&mut self, normal: Vec3, center: Vec3) {
        let n = normal.normalize();
        self.pitch = n.z.clamp(-1., 1.).asin().clamp(-1.5706, 1.5706);
        if n.x.abs() + n.y.abs() > 1e-4 {
            self.yaw = n.y.atan2(n.x);
        }
        self.target = center;
    }
}

struct Targets {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::Texture,
    tex_id: egui::TextureId,
}

type Buf = Option<(wgpu::Buffer, u32)>;

pub struct Viewport {
    bg_pipe: wgpu::RenderPipeline,
    mesh_pipe: wgpu::RenderPipeline,
    xray_pipe: wgpu::RenderPipeline,
    line_pipe: wgpu::RenderPipeline,
    overlay_pipe: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    vb: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    eb: Buf,
    lb: Buf,
    ob: Buf,
    targets: Option<Targets>,
    pub camera: Camera,
    last: Option<(Camera, [u32; 2], u64, ViewStyle)>,
    scene_gen: u64,
    pub bounds: Option<(Vec3, Vec3)>,
}

const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

/// Cor da interface (sRGB 0–255) para o shader. A textura é exibida pelo egui sem reconversão,
/// então os valores vão diretos, como as cores das faces.
fn lin(c: [u8; 3]) -> [f32; 4] {
    let f = |x: u8| x as f32 / 255.;
    [f(c[0]), f(c[1]), f(c[2]), 1.]
}

impl Viewport {
    pub fn new(rs: &egui_wgpu::RenderState) -> Self {
        let dev = &rs.device;
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("scc"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }],
        });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], ..Default::default() });
        let target = [Some(wgpu::ColorTargetState { format: FMT, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })];
        let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4];
        let line_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let pipe = |vs: &str, fs: &str, attrs: &[wgpu::VertexAttribute], stride: usize, topo, cmp, write| {
            let buffers = [Some(wgpu::VertexBufferLayout { array_stride: stride as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: attrs })];
            dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    buffers: if stride == 0 { &[] } else { &buffers },
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some(fs), targets: &target, compilation_options: Default::default() }),
                primitive: wgpu::PrimitiveState { topology: topo, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(write),
                    depth_compare: Some(cmp),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                multiview_mask: None,
                cache: None,
            })
        };
        use wgpu::CompareFunction as C;
        use wgpu::PrimitiveTopology as T;
        let ms = std::mem::size_of::<MeshVertex>();
        let ls = std::mem::size_of::<LineVertex>();
        Viewport {
            bg_pipe: pipe("vs_bg", "fs_bg", &[], 0, T::TriangleList, C::Always, false),
            mesh_pipe: pipe("vs_mesh", "fs_mesh", &mesh_attrs, ms, T::TriangleList, C::Less, true),
            xray_pipe: pipe("vs_mesh", "fs_mesh", &mesh_attrs, ms, T::TriangleList, C::Less, false),
            line_pipe: pipe("vs_line", "fs_line", &line_attrs, ls, T::LineList, C::LessEqual, false),
            overlay_pipe: pipe("vs_line", "fs_line", &line_attrs, ls, T::LineList, C::Always, false),
            ubuf,
            bind,
            vb: None,
            eb: None,
            lb: None,
            ob: None,
            targets: None,
            camera: Camera::default(),
            last: None,
            scene_gen: 0,
            bounds: None,
        }
    }

    pub fn set_scene(&mut self, rs: &egui_wgpu::RenderState, s: &Scene) {
        let dev = &rs.device;
        let mk = |data: &[u8], usage| dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: data, usage });
        let lines = |v: &Vec<LineVertex>| (!v.is_empty()).then(|| (mk(bytemuck::cast_slice(v), wgpu::BufferUsages::VERTEX), v.len() as u32));
        self.vb = (!s.indices.is_empty()).then(|| {
            (
                mk(bytemuck::cast_slice(&s.mesh), wgpu::BufferUsages::VERTEX),
                mk(bytemuck::cast_slice(&s.indices), wgpu::BufferUsages::INDEX),
                s.indices.len() as u32,
            )
        });
        self.eb = lines(&s.edges);
        self.lb = lines(&s.lines);
        self.ob = lines(&s.overlay);
        self.bounds = s.bounds;
        self.scene_gen += 1;
    }

    fn ensure_targets(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2]) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let dev = &rs.device;
        let ext = wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 };
        let tex = |fmt, samples, usage| {
            dev.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: ext,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage,
                view_formats: &[],
            })
        };
        let msaa = tex(FMT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default());
        let depth = tex(DEPTH, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default());
        let resolve = tex(FMT, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let view = resolve.create_view(&Default::default());
        let mut r = rs.renderer.write();
        let tex_id = match &self.targets {
            Some(t) => {
                r.update_egui_texture_from_wgpu_texture(dev, &view, wgpu::FilterMode::Linear, t.tex_id);
                t.tex_id
            }
            None => r.register_native_texture(dev, &view, wgpu::FilterMode::Linear),
        };
        self.targets = Some(Targets { size, msaa, depth, resolve, tex_id });
        self.last = None;
    }

    /// Renderiza (se necessário) e devolve a textura para exibir.
    pub fn render(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2], style: ViewStyle) -> egui::TextureId {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        self.ensure_targets(rs, size);
        let key = (self.camera, size, self.scene_gen, style);
        let t = self.targets.as_ref().unwrap();
        if self.last == Some(key) {
            return t.tex_id;
        }
        let vp = self.camera.view_proj(size[0] as f32 / size[1] as f32);
        let e = self.camera.eye();
        let mut bottom = lin(style.bg_bottom);
        bottom[3] = if style.gradient { 1. } else { 0. };
        let u = Uniforms {
            view_proj: vp.to_cols_array_2d(),
            eye: [e.x, e.y, e.z, 1.],
            params: [style.mode.code(), 0., 0., 0.],
            bg_top: lin(style.bg_top),
            bg_bottom: bottom,
            paper: [0.96, 0.96, 0.95, 1.],
        };
        rs.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));

        let mut enc = rs.device.create_command_encoder(&Default::default());
        {
            let view = t.resolve.create_view(&Default::default());
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    resolve_target: Some(&view),
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_pipeline(&self.bg_pipe);
            pass.draw(0..3, 0..1);

            let m = style.mode;
            let faces = !matches!(m, DisplayMode::Arame);
            let edges_always = matches!(m, DisplayMode::Arame | DisplayMode::RaioX);
            let edges = match m {
                DisplayMode::Sombreado => false,
                DisplayMode::SemiRenderizado => style.edges_in_render,
                _ => true,
            };
            if let (true, Some((vb, ib, n))) = (faces, &self.vb) {
                pass.set_pipeline(if m == DisplayMode::RaioX { &self.xray_pipe } else { &self.mesh_pipe });
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*n, 0, 0..1);
            }
            let mut lines = |b: &Buf, p: &wgpu::RenderPipeline| {
                if let Some((buf, n)) = b {
                    pass.set_pipeline(p);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..*n, 0..1);
                }
            };
            if edges {
                lines(&self.eb, if edges_always { &self.overlay_pipe } else { &self.line_pipe });
            }
            lines(&self.lb, &self.line_pipe);
            lines(&self.ob, &self.overlay_pipe);
        }
        rs.queue.submit([enc.finish()]);
        self.last = Some(key);
        t.tex_id
    }

    /// Projeta um ponto do mundo para a tela (dentro do retângulo da viewport).
    pub fn project(&self, rect: egui::Rect, p: Vec3) -> Option<egui::Pos2> {
        let vp = self.camera.view_proj(rect.width() / rect.height());
        let c = vp * p.extend(1.0);
        if c.w <= 0. {
            return None;
        }
        let n = c.truncate() / c.w;
        Some(egui::pos2(rect.left() + (n.x + 1.) / 2. * rect.width(), rect.top() + (1. - n.y) / 2. * rect.height()))
    }
}
