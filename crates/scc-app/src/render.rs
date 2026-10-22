//! Viewport 3D própria (wgpu): renderiza em textura offscreen (MSAA + profundidade) que o egui exibe.
//! Só renderiza de novo quando câmera, cena ou tamanho mudam.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, eye: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

struct MIn { @location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @location(2) col: vec4<f32> };
struct MOut { @builtin(position) clip: vec4<f32>, @location(0) n: vec3<f32>, @location(1) col: vec4<f32>, @location(2) wp: vec3<f32> };

@vertex fn vs_mesh(v: MIn) -> MOut {
    var o: MOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.n = v.nrm; o.col = v.col; o.wp = v.pos;
    return o;
}
@fragment fn fs_mesh(i: MOut) -> @location(0) vec4<f32> {
    let n = normalize(i.n);
    let v = normalize(u.eye.xyz - i.wp);
    let d = abs(dot(n, v));
    let k = normalize(vec3<f32>(0.3, 0.5, 1.0));
    let fill = max(dot(n, k), 0.0) * 0.25;
    let h = normalize(v + v);
    let spec = pow(max(abs(dot(n, h)), 0.0), 48.0) * 0.18;
    let c = i.col.rgb * (0.30 + 0.55 * d + fill) + vec3<f32>(spec);
    return vec4<f32>(c, i.col.a);
}

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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub col: [f32; 4],
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
}

/// Cena pronta para a GPU.
#[derive(Default)]
pub struct Scene {
    pub mesh: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// linhas com teste de profundidade (arestas, sketches)
    pub lines: Vec<LineVertex>,
    /// linhas sempre visíveis (sketch selecionado, eixos)
    pub overlay: Vec<LineVertex>,
    pub bounds: Option<(Vec3, Vec3)>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { target: Vec3::ZERO, yaw: -0.785, pitch: 0.6, dist: 300. }
    }
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        self.target + self.dist * Vec3::new(cp * self.yaw.cos(), cp * self.yaw.sin(), sp)
    }
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let up = if self.pitch.abs() > 1.55 { Vec3::new(-self.yaw.cos(), -self.yaw.sin(), 0.) * self.pitch.signum() } else { Vec3::Z };
        let view = glam::camera::rh::view::look_at_mat4(self.eye(), self.target, up);
        // wgpu: Z do NDC em [0, 1], Y para cima
        let proj = glam::camera::rh::proj::directx::perspective(0.6, aspect.max(0.01), self.dist * 0.01, self.dist * 50.);
        proj * view
    }
    /// Vetores direita/cima no mundo (para pan).
    pub fn basis(&self) -> (Vec3, Vec3) {
        let f = (self.target - self.eye()).normalize();
        let r = f.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
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
}

struct Targets {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::Texture,
    tex_id: egui::TextureId,
}

pub struct Viewport {
    mesh_pipe: wgpu::RenderPipeline,
    line_pipe: wgpu::RenderPipeline,
    overlay_pipe: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    vb: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    lb: Option<(wgpu::Buffer, u32)>,
    ob: Option<(wgpu::Buffer, u32)>,
    targets: Option<Targets>,
    pub camera: Camera,
    last: Option<(Camera, [u32; 2], u64)>,
    scene_gen: u64,
    pub bounds: Option<(Vec3, Vec3)>,
}

const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

impl Viewport {
    pub fn new(rs: &egui_wgpu::RenderState) -> Self {
        let dev = &rs.device;
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scc"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
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
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            ..Default::default()
        });
        let blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let target = [Some(wgpu::ColorTargetState { format: FMT, blend, write_mask: wgpu::ColorWrites::ALL })];
        let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];
        let line_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let pipe = |vs: &str, fs: &str, attrs: &[wgpu::VertexAttribute], stride: usize, topo, depth_cmp, depth_write| {
            dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: stride as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: attrs,
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    targets: &target,
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { topology: topo, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(depth_cmp),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                multiview_mask: None,
                cache: None,
            })
        };
        let mesh_pipe = pipe(
            "vs_mesh",
            "fs_mesh",
            &mesh_attrs,
            std::mem::size_of::<MeshVertex>(),
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::CompareFunction::Less,
            true,
        );
        let line_pipe = pipe(
            "vs_line",
            "fs_line",
            &line_attrs,
            std::mem::size_of::<LineVertex>(),
            wgpu::PrimitiveTopology::LineList,
            wgpu::CompareFunction::LessEqual,
            false,
        );
        let overlay_pipe = pipe(
            "vs_line",
            "fs_line",
            &line_attrs,
            std::mem::size_of::<LineVertex>(),
            wgpu::PrimitiveTopology::LineList,
            wgpu::CompareFunction::Always,
            false,
        );
        Viewport {
            mesh_pipe,
            line_pipe,
            overlay_pipe,
            ubuf,
            bind,
            vb: None,
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
        self.vb = (!s.indices.is_empty()).then(|| {
            (
                mk(bytemuck::cast_slice(&s.mesh), wgpu::BufferUsages::VERTEX),
                mk(bytemuck::cast_slice(&s.indices), wgpu::BufferUsages::INDEX),
                s.indices.len() as u32,
            )
        });
        self.lb = (!s.lines.is_empty()).then(|| (mk(bytemuck::cast_slice(&s.lines), wgpu::BufferUsages::VERTEX), s.lines.len() as u32));
        self.ob = (!s.overlay.is_empty()).then(|| (mk(bytemuck::cast_slice(&s.overlay), wgpu::BufferUsages::VERTEX), s.overlay.len() as u32));
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
    pub fn render(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2]) -> egui::TextureId {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        self.ensure_targets(rs, size);
        let key = (self.camera, size, self.scene_gen);
        let t = self.targets.as_ref().unwrap();
        if self.last == Some(key) {
            return t.tex_id;
        }
        let vp = self.camera.view_proj(size[0] as f32 / size[1] as f32);
        let e = self.camera.eye();
        let u = Uniforms { view_proj: vp.to_cols_array_2d(), eye: [e.x, e.y, e.z, 1.] };
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
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.16, g: 0.18, b: 0.21, a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.bind, &[]);
            if let Some((vb, ib, n)) = &self.vb {
                pass.set_pipeline(&self.mesh_pipe);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*n, 0, 0..1);
            }
            if let Some((lb, n)) = &self.lb {
                pass.set_pipeline(&self.line_pipe);
                pass.set_vertex_buffer(0, lb.slice(..));
                pass.draw(0..*n, 0..1);
            }
            if let Some((ob, n)) = &self.ob {
                pass.set_pipeline(&self.overlay_pipe);
                pass.set_vertex_buffer(0, ob.slice(..));
                pass.draw(0..*n, 0..1);
            }
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
