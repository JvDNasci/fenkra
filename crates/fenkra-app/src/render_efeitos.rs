// SPDX-License-Identifier: LGPL-2.1-or-later
//! B139: sombra suave, oclusão de ambiente e PBR na viewport.
//!
//! Como funciona:
//!
//! * **Sombra suave.** Antes da passada principal, a peça é desenhada só em profundidade do ponto
//!   de vista da luz principal (a mesma luz presa à câmera da iluminação realista), numa projeção
//!   ortográfica que abraça a caixa envolvente. Na passada principal cada pixel vai para o espaço da
//!   luz e compara a própria profundidade com a guardada: mais longe da luz é sombra. A borda suave
//!   vem de vários toques num disco (filtro de porcentagem mais próxima), cada toque já filtrado pela
//!   comparação bilinear da placa.
//! * **Oclusão de ambiente.** Uma passada só de profundidade, em meia resolução, guarda a cena vista
//!   da câmera. Cada pixel lança pontos num hemisfério em volta da normal e conta quantos ficam atrás
//!   do que a câmera vê: canto fechado escurece a luz de ambiente. Os pontos seguem uma espiral fixa,
//!   sem ruído, porque não há passada de borrão.
//! * **PBR.** Modelo de microfacetas (distribuição GGX, sombreamento de Smith e Fresnel de Schlick)
//!   para as luzes diretas e uma aproximação analítica da reflexão do ambiente. Não lê textura
//!   nenhuma, então vale no módulo base e nunca impede o programa de abrir.
//!
//! Sombra e oclusão moram num módulo de shader à parte, com um segundo grupo de recursos. Os pipelines
//! comuns não dependem dele: se a placa não aceitar alguma coisa, ou se o shader dos efeitos falhar,
//! a viewport segue desenhando com o módulo base, o nível cai para Rápido e o aviso sai uma vez.

use crate::i18n::tr;
use crate::render::{Camera, DisplayMode};
use eframe::egui;
use eframe::egui_wgpu::wgpu;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Nível de qualidade dos efeitos.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Nivel {
    /// sem sombra nem oclusão (só o PBR, que não custa passada)
    Rapido,
    #[default]
    Equilibrado,
    Alto,
}

impl Nivel {
    pub const TODOS: [Nivel; 3] = [Nivel::Rapido, Nivel::Equilibrado, Nivel::Alto];
    pub fn label(self) -> &'static str {
        match self {
            Nivel::Rapido => tr("Rápido"),
            Nivel::Equilibrado => tr("Equilibrado"),
            Nivel::Alto => tr("Alto"),
        }
    }
}

/// O que a pessoa pediu nas configurações gráficas.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Efeitos {
    pub nivel: Nivel,
    pub sombra: bool,
    pub oclusao: bool,
    pub pbr: bool,
    /// a pessoa já mexeu nos efeitos: a partida não os desliga mais sozinha no adaptador de software
    pub escolhidos: bool,
}

impl Default for Efeitos {
    fn default() -> Self {
        Efeitos { nivel: Nivel::Equilibrado, sombra: true, oclusao: true, pbr: true, escolhidos: false }
    }
}

impl Efeitos {
    /// Como nascem no modo leve e no adaptador de software.
    pub const DESLIGADOS: Efeitos = Efeitos { nivel: Nivel::Rapido, sombra: false, oclusao: false, pbr: false, escolhidos: false };
}

/// `FENKRA_DEMO_EFEITOS=desligados|rapido|equilibrado|alto|sem_pbr`: força os efeitos, para capturas
/// e medidas de antes e depois. Sem a variável, devolve o que veio.
pub fn demo(e: Efeitos) -> Efeitos {
    let Ok(v) = std::env::var("FENKRA_DEMO_EFEITOS") else { return e };
    let ligados = Efeitos::default();
    match v.as_str() {
        "desligados" => Efeitos::DESLIGADOS,
        "rapido" => Efeitos { nivel: Nivel::Rapido, ..ligados },
        "alto" => Efeitos { nivel: Nivel::Alto, ..ligados },
        "sem_pbr" => Efeitos { pbr: false, ..ligados },
        _ => ligados,
    }
}

/// O que a placa aceita para os efeitos que dependem de profundidade.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Capacidade {
    /// sombra e oclusão permitidas, com o mapa de sombra até `max_px`
    Completa { max_px: u32 },
    /// só o que não precisa de passada extra (PBR)
    SemProfundidade,
}

/// Menor mapa de sombra que ainda vale a pena.
const SOMBRA_MIN: u32 = 1024;

/// Decide a capacidade pelas flags de nível baixo e pelo maior lado de textura. Comparação de
/// profundidade é o único recurso fora do básico que a sombra usa; a oclusão só lê profundidade.
pub fn capacidade(flags: wgpu::DownlevelFlags, max_tex: u32) -> Capacidade {
    if !flags.contains(wgpu::DownlevelFlags::COMPARISON_SAMPLERS) || max_tex < SOMBRA_MIN {
        Capacidade::SemProfundidade
    } else {
        Capacidade::Completa { max_px: max_tex.min(4096) }
    }
}

/// Efeitos deste quadro, já com nível, modo de exibição e capacidade aplicados.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Ativos {
    /// lado do mapa de sombra (0 = sem sombra)
    pub sombra_px: u32,
    /// toques do filtro da sombra
    pub toques: u32,
    /// pontos da oclusão (0 = sem oclusão)
    pub ao_amostras: u32,
    pub pbr: bool,
}

impl Ativos {
    pub fn usa_profundidade(&self) -> bool {
        self.sombra_px > 0 || self.ao_amostras > 0
    }
}

/// O que vale neste quadro. Modo leve desliga tudo; os efeitos só entram nos sombreados (análise,
/// arame, raio-X e linhas ocultas medem ou mostram outra coisa); sombra e oclusão pedem iluminação
/// realista ou o semi-renderizado; ao navegar com "Simplificar ao girar" o nível cai para Rápido.
pub fn ativos(e: Efeitos, realista: bool, leve: bool, modo: DisplayMode, movendo: bool, cap: Capacidade) -> Ativos {
    let sombreado = matches!(modo, DisplayMode::SombreadoArestas | DisplayMode::Sombreado);
    let semi = modo == DisplayMode::SemiRenderizado;
    if leve || !((sombreado && realista) || semi) {
        return Ativos::default();
    }
    let mut a = Ativos { pbr: e.pbr && sombreado, ..Default::default() };
    let nivel = if movendo { Nivel::Rapido } else { e.nivel };
    let Capacidade::Completa { max_px } = cap else { return a };
    let (px, toques, ao) = match nivel {
        Nivel::Rapido => return a,
        Nivel::Equilibrado => (1024, 4, 8),
        Nivel::Alto => (2048, 12, 16),
    };
    if e.sombra {
        a.sombra_px = px.min(max_px);
        a.toques = toques;
    }
    if e.oclusao {
        a.ao_amostras = ao;
    }
    a
}

/// Direção da luz principal presa à câmera: acima e à esquerda de quem olha, como em
/// `shade_realistic`. Devolve a direção que vai do modelo para a luz.
pub fn direcao_da_luz(cam: &Camera) -> Vec3 {
    let v = (cam.eye() - cam.target).normalize_or(Vec3::Z);
    let right = Vec3::Z.cross(v).try_normalize().unwrap_or(Vec3::X);
    let up = v.cross(right);
    (v * 0.55 + up * 0.60 - right * 0.55).normalize()
}

/// Projeção da luz: ortográfica, olhando o centro da caixa envolvente pela direção da luz, com a
/// esfera que contém a caixa inteira dentro do volume. Devolve a matriz e o raio da esfera.
pub fn projecao_da_luz(dir: Vec3, b: (Vec3, Vec3)) -> (Mat4, f32) {
    let c = (b.0 + b.1) * 0.5;
    let r = ((b.1 - b.0).length() * 0.5).max(1.);
    let olho = c + dir * r * 2.;
    let cima = if dir.z.abs() > 0.95 { Vec3::Y } else { Vec3::Z };
    let view = glam::camera::rh::view::look_at_mat4(olho, c, cima);
    let proj = glam::camera::rh::proj::directx::orthographic(-r, r, -r, r, r * 0.5, r * 3.5);
    (proj * view, r)
}

/// Valores dos efeitos que vão para o uniforme do shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformeEfeitos {
    pub luz_vp: [[f32; 4]; 4],
    pub luz: [f32; 4],
    pub efeitos: [f32; 4],
    pub efeitos2: [f32; 4],
    pub prof: [f32; 4],
}

/// Monta o uniforme. Sem caixa envolvente não há sombra (não há onde pôr a luz). `luz_fixa`: a luz
/// principal da cena (B182) está parada no mundo, e a sombra vem dessa direção.
pub fn uniforme(a: Ativos, cam: &Camera, bounds: Option<(Vec3, Vec3)>, luz_fixa: Option<Vec3>) -> UniformeEfeitos {
    let dir = luz_fixa.and_then(|d| d.try_normalize()).unwrap_or_else(|| direcao_da_luz(cam));
    let (luz_vp, r) = bounds.map_or((Mat4::IDENTITY, 1.), |b| projecao_da_luz(dir, b));
    let sombra = a.sombra_px > 0 && bounds.is_some();
    let px = a.sombra_px.max(1) as f32;
    // recuo pela normal: um texel e meio do mapa no mundo, contra a acne nas faces viradas para a luz
    let texel_mundo = 2. * r / px;
    // raio da oclusão: uma fração do tamanho do modelo, nem tão pequeno que some, nem tão grande que borra
    let ao_raio = bounds.map_or(10., |b| ((b.1 - b.0).length() * 0.035).max(0.5));
    let (perto, longe, orto) = if cam.ortho { (-cam.dist * 50., cam.dist * 50., 1.) } else { (cam.dist * 0.01, cam.dist * 50., 0.) };
    UniformeEfeitos {
        luz_vp: luz_vp.to_cols_array_2d(),
        luz: [dir.x, dir.y, dir.z, 0.0015],
        efeitos: [if sombra { 1. } else { 0. }, a.ao_amostras as f32, if a.pbr { 1. } else { 0. }, a.toques.max(1) as f32],
        efeitos2: [1. / px, ao_raio, 0., texel_mundo * 1.5],
        prof: [perto, longe, orto, 0.],
    }
}

// ---------------------------------------------------------------- B182: cena de apresentação

/// O que a vista precisa da cena de apresentação, já traduzido para números (cabe no estilo da vista,
/// que é copiável e comparado a cada quadro para saber se redesenha).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CenaVista {
    /// luz principal fixa no mundo (direção para a luz); `None` = presa à câmera
    pub luz_fixa: Option<[f32; 3]>,
    /// cor × intensidade da luz principal (1, 1, 1 = de fábrica; 0 = desligada)
    pub luz_cor: [f32; 3],
    /// força da luz de ambiente
    pub ambiente: f32,
    /// imagem panorâmica de fundo: giro (rad) e exposição. A imagem em si sobe pela viewport.
    pub panoramica: Option<[f32; 2]>,
    pub chao: bool,
    /// 0 = sem reflexo
    pub reflexo: f32,
    pub sombra_no_chao: bool,
}

impl Default for CenaVista {
    fn default() -> Self {
        CenaVista { luz_fixa: None, luz_cor: [1.; 3], ambiente: 1., panoramica: None, chao: false, reflexo: 0., sombra_no_chao: false }
    }
}

impl CenaVista {
    /// Tradução da cena do documento. A cor da luz vale em espaço linear, como a soma da iluminação.
    pub fn de(c: &fenkra_core::cena::Cena) -> CenaVista {
        use fenkra_core::cena::{Fundo, TipoLuz};
        let a = &c.ambiente;
        let lin = |x: u8| (x as f32 / 255.).powf(2.2);
        let (luz_fixa, luz_cor) = match c.luz_principal() {
            Some(l) => ((l.tipo == TipoLuz::Direcional).then_some(l.direcao), l.cor.map(|x| lin(x) * l.intensidade.max(0.))),
            None => (None, [0.; 3]),
        };
        CenaVista {
            luz_fixa,
            luz_cor,
            ambiente: a.ambiente.max(0.),
            panoramica: match &a.fundo {
                Fundo::Panoramica { caminho, rotacao_graus, exposicao } if !caminho.trim().is_empty() => Some([rotacao_graus.to_radians(), exposicao.max(0.)]),
                _ => None,
            },
            chao: a.chao,
            reflexo: if a.chao && a.reflexo_ligado { a.reflexo.clamp(0., 1.) } else { 0. },
            sombra_no_chao: a.chao && a.sombra_no_chao,
        }
    }
}

/// O que da cena vale neste quadro.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct CenaQuadro {
    pub chao: bool,
    /// força do reflexo (0 = a peça espelhada não é desenhada)
    pub reflexo: f32,
    /// o chão lê o mapa de sombra
    pub sombra_no_chao: bool,
    /// as faces refletem a imagem panorâmica (senão, só a cor média dela)
    pub reflexo_panoramica: bool,
}

/// Regras de custo da cena, como as do B139: chão, reflexo e sombra no chão só nos modos sombreados e
/// no semi-renderizado; o modo leve tira tudo; o nível Rápido (também ao girar com "Simplificar ao
/// girar") tira o reflexo, que redesenha a peça, e a leitura da imagem panorâmica nas faces. O chão em
/// si são dois triângulos e fica. A sombra no chão só existe quando a sombra suave do quadro existe.
/// Com a câmera abaixo do chão não há chão nem reflexo.
pub fn cena_do_quadro(c: &CenaVista, e: Efeitos, a: Ativos, leve: bool, modo: DisplayMode, movendo: bool, abaixo_do_chao: bool) -> CenaQuadro {
    let sombreado = matches!(modo, DisplayMode::SombreadoArestas | DisplayMode::Sombreado | DisplayMode::SemiRenderizado);
    if leve || !sombreado {
        return CenaQuadro::default();
    }
    let rapido = movendo || e.nivel == Nivel::Rapido;
    let chao = c.chao && !abaixo_do_chao;
    CenaQuadro {
        chao,
        reflexo: if chao && !rapido { c.reflexo } else { 0. },
        sombra_no_chao: chao && c.sombra_no_chao && a.sombra_px > 0,
        reflexo_panoramica: c.panoramica.is_some() && !rapido,
    }
}

/// Plano do chão para a caixa envolvente: centro em XY, altura (um pouco abaixo da base, para não
/// brigar com a face de baixo) e meio lado.
pub fn plano_do_chao(b: (Vec3, Vec3)) -> [f32; 4] {
    let diag = (b.1 - b.0).length().max(1.);
    let c = (b.0 + b.1) * 0.5;
    [c.x, c.y, b.0.z - diag * 5e-4, diag * 0.9]
}

/// Valores da cena que vão para o uniforme do shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformeCena {
    pub inv_vp: [[f32; 4]; 4],
    /// x = imagem panorâmica, y = giro (rad), z = exposição, w = luz de ambiente
    pub cena: [f32; 4],
    /// cor média da imagem panorâmica (valores de tela)
    pub cena_media: [f32; 4],
    /// rgb = cor da luz principal; w = 1 usa a direção do uniforme da luz (luz fixa)
    pub luz_cor: [f32; 4],
    /// xy = centro, z = altura, w = meio lado
    pub chao: [f32; 4],
    /// x = chão ligado, y = reflexo, z = sombra no chão, w = faces refletem a imagem panorâmica
    pub chao2: [f32; 4],
}

pub fn uniforme_cena(c: &CenaVista, q: CenaQuadro, vp: Mat4, bounds: Option<(Vec3, Vec3)>, media: [u8; 3]) -> UniformeCena {
    let b = |x: bool| if x { 1. } else { 0. };
    let [giro, exp] = c.panoramica.unwrap_or([0., 1.]);
    UniformeCena {
        inv_vp: vp.inverse().to_cols_array_2d(),
        cena: [b(c.panoramica.is_some()), giro, exp, c.ambiente],
        cena_media: [media[0] as f32 / 255., media[1] as f32 / 255., media[2] as f32 / 255., 1.],
        luz_cor: [c.luz_cor[0], c.luz_cor[1], c.luz_cor[2], b(c.luz_fixa.is_some())],
        chao: bounds.map_or([0., 0., 0., 1.], plano_do_chao),
        chao2: [b(q.chao && bounds.is_some()), q.reflexo, b(q.sombra_no_chao), b(q.reflexo_panoramica)],
    }
}

/// Funções vazias do módulo base: sem sombra e sem oclusão.
pub const STUB_WGSL: &str = r#"
fn sombra_em(wp: vec3<f32>, n: vec3<f32>) -> f32 { return 1.0; }
fn oclusao_em(wp: vec3<f32>, n: vec3<f32>) -> f32 { return 1.0; }
"#;

/// As mesmas funções no módulo dos efeitos, com o grupo 1 (mapa de sombra, amostrador de
/// comparação e profundidade da câmera em meia resolução).
pub const EFEITOS_WGSL: &str = r#"
@group(1) @binding(0) var sombra_tex: texture_depth_2d;
@group(1) @binding(1) var sombra_cmp: sampler_comparison;
// profundidade da câmera em meia resolução, empacotada em RGBA8: o GLSL do OpenGL ES só lê textura
// de profundidade como sombra (comparação), então a oclusão lê uma textura de cor comum
@group(1) @binding(2) var ao_tex: texture_2d<f32>;
@group(1) @binding(3) var ao_samp: sampler;

// fração iluminada pela luz principal: toques numa espiral de ângulo áureo dentro de um disco
fn sombra_em(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    if (u.efeitos.x < 0.5) { return 1.0; }
    let c = u.luz_vp * vec4<f32>(wp + n * u.efeitos2.w, 1.0);
    let ndc = c.xyz / c.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0 || ndc.z > 1.0) { return 1.0; }
    let toques = max(i32(u.efeitos.w + 0.5), 1);
    let raio = u.efeitos2.x * select(1.5, 3.0, toques > 4);
    let ref_z = ndc.z - u.luz.w;
    var s = 0.0;
    for (var k = 0; k < 16; k = k + 1) {
        if (k >= toques) { break; }
        let fk = f32(k) + 0.5;
        let r = sqrt(fk / f32(toques)) * raio;
        let a = fk * 2.39996323;
        s = s + textureSampleCompareLevel(sombra_tex, sombra_cmp, uv + vec2<f32>(cos(a), sin(a)) * r, ref_z);
    }
    return s / f32(toques);
}

// profundidade de NDC para distância ao longo da vista (perspectiva ou ortográfica)
fn prof_linear(z: f32) -> f32 {
    let n = u.prof.x;
    let f = u.prof.y;
    if (u.prof.z > 0.5) { return n + z * (f - n); }
    return n * f / max(f - z * (f - n), 1e-6);
}

// luz de ambiente que chega ao ponto: 1 aberto, menos nos cantos fechados
fn oclusao_em(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    let amostras = i32(u.efeitos.y + 0.5);
    if (amostras < 1) { return 1.0; }
    let r = u.efeitos2.y;
    // base ortonormal em volta da normal, sem divisão por zero (método de Frisvad revisado)
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let bt = vec3<f32>(b, s + n.y * n.y * a, -n.y);
    let c0 = u.view_proj * vec4<f32>(wp, 1.0);
    let d0 = prof_linear(c0.z / c0.w);
    var occ = 0.0;
    for (var k = 0; k < 16; k = k + 1) {
        if (k >= amostras) { break; }
        let fk = f32(k) + 0.5;
        let fr = fk / f32(amostras);
        let z = 1.0 - fr * 0.85;
        let rr = sqrt(max(1.0 - z * z, 0.0));
        let ang = fk * 2.39996323;
        let esc = mix(0.2, 1.0, fr * fr);
        let kv = vec3<f32>(cos(ang) * rr, sin(ang) * rr, z) * esc;
        let p = wp + n * (r * 0.04) + (t * kv.x + bt * kv.y + n * kv.z) * r;
        let c = u.view_proj * vec4<f32>(p, 1.0);
        if (c.w <= 0.0) { continue; }
        let ndc = c.xyz / c.w;
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) { continue; }
        let dc = prof_linear(desempacotar(textureSampleLevel(ao_tex, ao_samp, uv, 0.0)));
        let ds = prof_linear(ndc.z);
        // o que está muito à frente (outra peça longe) não fecha este canto
        let perto = smoothstep(0.0, 1.0, r / max(abs(d0 - dc), 1e-4));
        occ = occ + select(0.0, 1.0, dc < ds - r * 0.03) * perto;
    }
    return 1.0 - 0.85 * occ / f32(amostras);
}
"#;

/// Marca no shader base onde entram as funções de sombra e oclusão.
pub const MARCA: &str = "//@EFEITOS@";

/// Espera um futuro que no nativo já vem pronto (o resultado de um escopo de erro).
fn esperar<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// Roda `f` dentro de escopos de erro de validação e interno: o erro volta como texto em vez de ir
/// para o tratador geral (que derrubaria o programa).
pub fn protegido<T>(dev: &wgpu::Device, f: impl FnOnce() -> T) -> Result<T, String> {
    let validacao = dev.push_error_scope(wgpu::ErrorFilter::Validation);
    let interno = dev.push_error_scope(wgpu::ErrorFilter::Internal);
    let v = f();
    let e1 = esperar(interno.pop());
    let e2 = esperar(validacao.pop());
    match e1.or(e2) {
        Some(e) => Err(e.to_string()),
        None => Ok(v),
    }
}

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Recursos de placa da sombra e da oclusão. Criados só quando um efeito com profundidade é pedido.
pub struct Recursos {
    module: wgpu::ShaderModule,
    bgl1: wgpu::BindGroupLayout,
    layout: wgpu::PipelineLayout,
    cmp: wgpu::Sampler,
    ponto: wgpu::Sampler,
    vazia: wgpu::TextureView,
    sombra: Option<(u32, wgpu::TextureView)>,
    /// cor empacotada e profundidade da passada da câmera
    ao: Option<([u32; 2], wgpu::TextureView, wgpu::TextureView)>,
    vazia_cor: wgpu::TextureView,
    bind: Option<wgpu::BindGroup>,
    /// passadas só de profundidade: luz (malha comum e cópias) e câmera em meia resolução
    p_sombra: wgpu::RenderPipeline,
    p_sombra_i: wgpu::RenderPipeline,
    p_ao: wgpu::RenderPipeline,
    p_ao_i: wgpu::RenderPipeline,
    pipes: Vec<crate::render::Pipes>,
}

fn textura_prof(dev: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    dev.create_texture(&wgpu::TextureDescriptor {
        label: Some("efeitos_prof"),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
    .create_view(&Default::default())
}

/// Formato da profundidade empacotada da oclusão: RGBA8 desenhável em qualquer placa.
const AO_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn textura_ao(dev: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    dev.create_texture(&wgpu::TextureDescriptor {
        label: Some("oclusao_prof"),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: AO_FMT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
    .create_view(&Default::default())
}

/// Pipeline só de profundidade (sem cor), uma amostra; com `cor`, grava também a profundidade
/// empacotada nesse formato.
#[allow(clippy::too_many_arguments)]
fn so_profundidade(dev: &wgpu::Device, module: &wgpu::ShaderModule, layout: &wgpu::PipelineLayout, vs: &str, fs: Option<&str>, copias: bool, inclinacao: f32) -> wgpu::RenderPipeline {
    let alvo = [Some(wgpu::ColorTargetState { format: AO_FMT, blend: None, write_mask: wgpu::ColorWrites::ALL })];
    let ms = std::mem::size_of::<crate::render::MeshVertex>() as u64;
    let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4];
    let inst_attrs = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];
    let pos_attr = wgpu::vertex_attr_array![0 => Float32x3];
    let nrm_attr = wgpu::vertex_attr_array![1 => Float32x3];
    let vb = |attributes| wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes };
    let ib = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<crate::instancias::InstanciaGpu>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &inst_attrs };
    let comum = [Some(wgpu::VertexBufferLayout { array_stride: ms, step_mode: wgpu::VertexStepMode::Vertex, attributes: &mesh_attrs })];
    let de_copia = [Some(vb(&pos_attr)), Some(vb(&nrm_attr)), Some(ib)];
    dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(vs),
        layout: Some(layout),
        vertex: wgpu::VertexState { module, entry_point: Some(vs), buffers: if copias { &de_copia } else { &comum }, compilation_options: Default::default() },
        fragment: fs.map(|fs| wgpu::FragmentState { module, entry_point: Some(fs), targets: &alvo, compilation_options: Default::default() }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: wgpu::DepthBiasState { constant: 2, slope_scale: inclinacao, clamp: 0.0 },
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

impl Recursos {
    /// Compila o módulo dos efeitos e as passadas de profundidade. Qualquer erro da placa volta como
    /// `Err`, e a viewport segue sem os efeitos.
    pub fn criar(dev: &wgpu::Device, bgl0: &wgpu::BindGroupLayout, base: &wgpu::ShaderModule, base_layout: &wgpu::PipelineLayout, fonte: &str) -> Result<Recursos, String> {
        protegido(dev, || {
            let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fenkra_efeitos"), source: wgpu::ShaderSource::Wgsl(fonte.into()) });
            let prof = |binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                count: None,
            };
            let cor = wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                count: None,
            };
            let bgl1 = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("efeitos"),
                entries: &[
                    prof(0),
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), count: None },
                    cor,
                    wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering), count: None },
                ],
            });
            let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("efeitos"), bind_group_layouts: &[Some(bgl0), Some(&bgl1)], ..Default::default() });
            let cmp = dev.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("sombra"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            });
            let ponto = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("oclusao"), address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, ..Default::default() });
            Recursos {
                ponto,
                p_sombra: so_profundidade(dev, base, base_layout, "vs_sombra", None, false, 2.0),
                p_sombra_i: so_profundidade(dev, base, base_layout, "vs_sombra_i", None, true, 2.0),
                p_ao: so_profundidade(dev, base, base_layout, "vs_mesh", Some("fs_corte"), false, 0.0),
                p_ao_i: so_profundidade(dev, base, base_layout, "vs_mesh_i", Some("fs_corte"), true, 0.0),
                vazia: textura_prof(dev, 1, 1),
                vazia_cor: textura_ao(dev, 1, 1),
                module,
                bgl1,
                layout,
                cmp,
                sombra: None,
                ao: None,
                bind: None,
                pipes: vec![],
            }
        })
    }

    /// Garante texturas e pipelines para estes efeitos, este tamanho de vista e esta contagem de amostras.
    pub fn garantir(&mut self, dev: &wgpu::Device, a: Ativos, vista: [u32; 2], amostras: u32) -> Result<(), String> {
        protegido(dev, || {
            let mut mudou = false;
            let px = a.sombra_px.max(1);
            if self.sombra.as_ref().is_none_or(|s| s.0 != px) {
                self.sombra = Some((px, textura_prof(dev, px, px)));
                mudou = true;
            }
            let meia = if a.ao_amostras > 0 { [vista[0].div_ceil(2).max(1), vista[1].div_ceil(2).max(1)] } else { [1, 1] };
            if self.ao.as_ref().is_none_or(|s| s.0 != meia) {
                self.ao = Some((meia, textura_ao(dev, meia[0], meia[1]), textura_prof(dev, meia[0], meia[1])));
                mudou = true;
            }
            if mudou || self.bind.is_none() {
                let s = &self.sombra.as_ref().unwrap().1;
                let o = &self.ao.as_ref().unwrap().1;
                self.bind = Some(dev.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("efeitos"),
                    layout: &self.bgl1,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(if a.sombra_px > 0 { s } else { &self.vazia }) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.cmp) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(if a.ao_amostras > 0 { o } else { &self.vazia_cor }) },
                        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.ponto) },
                    ],
                }));
            }
            if !self.pipes.iter().any(|p| p.samples == amostras) {
                self.pipes.push(crate::render::make_pipes(dev, &self.module, &self.layout, amostras));
            }
        })
    }

    pub fn bind(&self) -> Option<&wgpu::BindGroup> {
        self.bind.as_ref()
    }

    pub fn pipes(&self, amostras: u32) -> Option<&crate::render::Pipes> {
        self.pipes.iter().find(|p| p.samples == amostras)
    }

    /// Passadas de profundidade (luz e câmera) antes da passada principal.
    pub fn passadas(&self, enc: &mut wgpu::CommandEncoder, a: Ativos, bind0: &wgpu::BindGroup, malha: Option<(&wgpu::Buffer, &wgpu::Buffer, u32)>, inst: &crate::instancias::Instanciador) {
        let mut passada = |alvo: &wgpu::TextureView, cor: Option<&wgpu::TextureView>, p: &wgpu::RenderPipeline, pi: &wgpu::RenderPipeline, nome: &str| {
            // a passada da sombra não tem anexo de cor nenhum: um anexo vazio na lista já não casa
            // com o pipeline sem alvos
            let anexo = [cor.map(|view| wgpu::RenderPassColorAttachment { view, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store } })];
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(nome),
                color_attachments: if cor.is_some() { &anexo } else { &[] },
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: alvo,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: if cor.is_some() { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store } }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, bind0, &[]);
            if let Some((vb, ib, n)) = malha {
                pass.set_pipeline(p);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }
            pass.set_pipeline(pi);
            inst.faces(&mut pass);
        };
        if let (true, Some((_, v))) = (a.sombra_px > 0, &self.sombra) {
            passada(v, None, &self.p_sombra, &self.p_sombra_i, "sombra");
        }
        if let (true, Some((_, c, z))) = (a.ao_amostras > 0, &self.ao) {
            passada(z, Some(c), &self.p_ao, &self.p_ao_i, "oclusao");
        }
    }
}

/// Controles na seção Gráficos das configurações: nível e um interruptor por efeito, com dica.
pub fn ui(ui: &mut egui::Ui, e: &mut Efeitos, realista: bool) {
    let antes = *e;
    ui.add_enabled_ui(realista, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Efeitos da vista")).on_hover_text(tr("Quanto a vista gasta com sombra e oclusão. Rápido desliga as duas; Alto usa um mapa de sombra maior, borda mais suave e mais pontos de oclusão."));
            egui::ComboBox::from_id_salt("nivel_efeitos").selected_text(e.nivel.label()).show_ui(ui, |ui| {
                for n in Nivel::TODOS {
                    ui.selectable_value(&mut e.nivel, n, n.label());
                }
            });
        });
        ui.checkbox(&mut e.sombra, tr("Sombra suave")).on_hover_text(tr("A luz principal projeta sombra de borda suave na própria peça. Custa uma passada a mais por quadro."));
        ui.checkbox(&mut e.oclusao, tr("Oclusão de ambiente")).on_hover_text(tr("Escurece cantos e frestas onde a luz do ambiente chega menos. Custa uma passada em meia resolução."));
        ui.checkbox(&mut e.pbr, tr("Material físico (PBR)")).on_hover_text(tr("Brilho e reflexo calculados pela rugosidade e pela metalicidade do material. Não custa passada a mais."));
    });
    ui.label(egui::RichText::new(tr("Precisam da iluminação realista. No modo leve e em placa sem suporte ficam desligados; ao girar com \"Simplificar ao girar\" a vista usa o nível Rápido.")).weak().small());
    if *e != antes {
        e.escolhidos = true;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const TUDO: Capacidade = Capacidade::Completa { max_px: 4096 };

    #[test]
    fn modo_leve_e_nivel_rapido_desligam_sombra_e_oclusao() {
        let e = Efeitos::default();
        assert_eq!(ativos(e, true, true, DisplayMode::SombreadoArestas, false, TUDO), Ativos::default());
        let r = ativos(Efeitos { nivel: Nivel::Rapido, ..e }, true, false, DisplayMode::SombreadoArestas, false, TUDO);
        assert_eq!((r.sombra_px, r.ao_amostras), (0, 0));
        assert!(r.pbr, "o PBR não custa passada e fica no Rápido");
        let n = ativos(e, true, false, DisplayMode::SombreadoArestas, false, TUDO);
        assert!(n.sombra_px == 1024 && n.ao_amostras == 8 && n.toques == 4);
        let a = ativos(Efeitos { nivel: Nivel::Alto, ..e }, true, false, DisplayMode::Sombreado, false, TUDO);
        assert!(a.sombra_px == 2048 && a.ao_amostras == 16 && a.toques == 12);
    }

    #[test]
    fn sem_recurso_o_nivel_cai_para_rapido() {
        let e = Efeitos { nivel: Nivel::Alto, ..Default::default() };
        let r = ativos(e, true, false, DisplayMode::SombreadoArestas, false, Capacidade::SemProfundidade);
        assert!(!r.usa_profundidade() && r.pbr);
        assert_eq!(capacidade(wgpu::DownlevelFlags::empty(), 8192), Capacidade::SemProfundidade);
        assert_eq!(capacidade(wgpu::DownlevelFlags::COMPARISON_SAMPLERS, 512), Capacidade::SemProfundidade);
        // OpenGL ES 3 dá 2048: a sombra do Alto cabe, e uma placa menor recorta o mapa
        assert_eq!(capacidade(wgpu::DownlevelFlags::COMPARISON_SAMPLERS, 2048), Capacidade::Completa { max_px: 2048 });
        let c = ativos(e, true, false, DisplayMode::SombreadoArestas, false, Capacidade::Completa { max_px: 1024 });
        assert_eq!(c.sombra_px, 1024);
    }

    #[test]
    fn efeitos_so_nos_sombreados_e_navegando_cai() {
        let e = Efeitos::default();
        for m in [DisplayMode::Arame, DisplayMode::RaioX, DisplayMode::LinhasOcultas, DisplayMode::Zebra, DisplayMode::Curvatura, DisplayMode::Dfm] {
            assert_eq!(ativos(e, true, false, m, false, TUDO), Ativos::default(), "{m:?}");
        }
        // sem iluminação realista, só o semi-renderizado leva sombra
        assert_eq!(ativos(e, false, false, DisplayMode::Sombreado, false, TUDO), Ativos::default());
        let s = ativos(e, false, false, DisplayMode::SemiRenderizado, false, TUDO);
        assert!(s.sombra_px > 0 && !s.pbr);
        assert!(!ativos(e, true, false, DisplayMode::Sombreado, true, TUDO).usa_profundidade());
        let so = ativos(Efeitos { oclusao: false, ..e }, true, false, DisplayMode::Sombreado, false, TUDO);
        assert!(so.sombra_px > 0 && so.ao_amostras == 0);
    }

    #[test]
    fn a_caixa_inteira_cabe_no_volume_da_luz() {
        let b = (Vec3::new(-40., -10., 0.), Vec3::new(60., 30., 25.));
        for (yaw, pitch) in [(-0.785f32, 0.6f32), (2.0, -0.3), (0.3, 1.5), (-2.5, 0.05)] {
            let cam = Camera { target: Vec3::new(10., 10., 12.), yaw, pitch, dist: 300., ortho: false };
            let dir = direcao_da_luz(&cam);
            assert!((dir.length() - 1.).abs() < 1e-4);
            // a luz fica do lado de quem olha: ilumina o que a câmera vê
            assert!(dir.dot((cam.eye() - cam.target).normalize()) > 0.3);
            let (m, _) = projecao_da_luz(dir, b);
            for i in 0..8 {
                let p = Vec3::new(if i & 1 == 0 { b.0.x } else { b.1.x }, if i & 2 == 0 { b.0.y } else { b.1.y }, if i & 4 == 0 { b.0.z } else { b.1.z });
                let c = m.project_point3(p);
                assert!(c.x.abs() <= 1. && c.y.abs() <= 1. && (0. ..=1.).contains(&c.z), "canto {p} fora: {c}");
            }
        }
    }

    #[test]
    fn uniforme_desligado_nao_muda_o_shader() {
        let cam = Camera::default();
        let u = uniforme(Ativos::default(), &cam, Some((Vec3::ZERO, Vec3::splat(10.))), None);
        assert_eq!((u.efeitos[0], u.efeitos[1], u.efeitos[2]), (0., 0., 0.));
        // sem caixa não há sombra, mesmo pedida
        let a = Ativos { sombra_px: 1024, toques: 4, ao_amostras: 8, pbr: true };
        assert_eq!(uniforme(a, &cam, None, None).efeitos[0], 0.);
        assert_eq!(uniforme(a, &cam, Some((Vec3::ZERO, Vec3::ONE)), None).efeitos[0], 1.);
    }

    /// B182: a luz fixa da cena dá a direção da sombra, e a sombra não gira com a vista.
    #[test]
    fn luz_fixa_da_cena_manda_na_sombra() {
        let a = Ativos { sombra_px: 1024, toques: 4, ao_amostras: 0, pbr: true };
        let b = Some((Vec3::ZERO, Vec3::splat(10.)));
        let fixa = Vec3::new(-0.6, -0.3, 0.75);
        let u1 = uniforme(a, &Camera::default(), b, Some(fixa));
        let u2 = uniforme(a, &Camera { yaw: 2.0, ..Camera::default() }, b, Some(fixa));
        assert_eq!(u1.luz_vp, u2.luz_vp);
        let d = Vec3::from_slice(&u1.luz[..3]);
        assert!((d - fixa.normalize()).length() < 1e-5);
        // presa à câmera, girar a vista gira a luz
        assert_ne!(uniforme(a, &Camera::default(), b, None).luz_vp, uniforme(a, &Camera { yaw: 2.0, ..Camera::default() }, b, None).luz_vp);
    }

    /// B182: cena de fábrica não muda o shader; cada efeito da cena tem interruptor e o nível Rápido
    /// (ou girar com "Simplificar ao girar") tira o reflexo e a leitura da imagem nas faces.
    #[test]
    fn cena_respeita_interruptores_e_nivel_rapido() {
        use fenkra_core::cena::{Cena, Fundo};
        let fabrica = CenaVista::de(&Cena::default());
        assert_eq!(fabrica, CenaVista::default());
        let e = Efeitos::default();
        let m = DisplayMode::SombreadoArestas;
        let a = ativos(e, true, false, m, false, TUDO);
        assert_eq!(cena_do_quadro(&fabrica, e, a, false, m, false, false), CenaQuadro::default());
        let mut c = Cena::default();
        c.ambiente.chao = true;
        c.ambiente.reflexo_ligado = true;
        c.ambiente.reflexo = 0.4;
        c.ambiente.fundo = Fundo::Panoramica { caminho: "a.png".into(), rotacao_graus: 90., exposicao: 1. };
        let cv = CenaVista::de(&c);
        let tudo = cena_do_quadro(&cv, e, a, false, m, false, false);
        assert!(tudo.chao && tudo.reflexo == 0.4 && tudo.sombra_no_chao && tudo.reflexo_panoramica);
        // interruptores: sem reflexo ligado não há segunda passada da peça
        let mut sem = c.clone();
        sem.ambiente.reflexo_ligado = false;
        sem.ambiente.sombra_no_chao = false;
        let q = cena_do_quadro(&CenaVista::de(&sem), e, a, false, m, false, false);
        assert!(q.chao && q.reflexo == 0. && !q.sombra_no_chao);
        // Rápido: sem sombra (B139), sem reflexo, sem imagem nas faces; o chão fica
        let er = Efeitos { nivel: Nivel::Rapido, ..e };
        let ar = ativos(er, true, false, m, false, TUDO);
        let r = cena_do_quadro(&cv, er, ar, false, m, false, false);
        assert!(r.chao && r.reflexo == 0. && !r.sombra_no_chao && !r.reflexo_panoramica);
        assert_eq!(cena_do_quadro(&cv, e, ativos(e, true, false, m, true, TUDO), false, m, true, false).reflexo, 0.);
        // modo leve, modos de análise e câmera abaixo do chão
        assert_eq!(cena_do_quadro(&cv, e, a, true, m, false, false), CenaQuadro::default());
        assert!(!cena_do_quadro(&cv, e, a, false, DisplayMode::Zebra, false, false).chao);
        let baixo = cena_do_quadro(&cv, e, a, false, m, false, true);
        assert!(!baixo.chao && baixo.reflexo == 0.);
        // o chão fica sob a peça, abaixo da base
        let p = plano_do_chao((Vec3::new(-10., -10., 5.), Vec3::new(10., 30., 25.)));
        assert!(p[0] == 0. && p[1] == 10. && p[2] < 5. && p[3] > 20.);
    }

    /// O shader base e o dos efeitos compilam (naga) e criam pipelines em cada placa disponível,
    /// inclusive OpenGL quando houver: o efeito não pode ser o motivo de o programa não abrir.
    #[test]
    fn shaders_validam_em_cada_backend() {
        let mut provados = 0;
        for bk in [wgpu::Backends::PRIMARY, wgpu::Backends::GL] {
            let inst = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: bk, ..wgpu::InstanceDescriptor::new_without_display_handle() });
            let Ok(ad) = esperar(inst.request_adapter(&wgpu::RequestAdapterOptions::default())) else { continue };
            let limites = if ad.get_info().backend == wgpu::Backend::Gl { wgpu::Limits::downlevel_webgl2_defaults() } else { wgpu::Limits::default() };
            let Ok((dev, q)) = esperar(ad.request_device(&wgpu::DeviceDescriptor { required_limits: limites, ..Default::default() })) else { continue };
            let r = crate::render::testar_pipelines(&dev, &q, capacidade(ad.get_downlevel_capabilities().flags, dev.limits().max_texture_dimension_2d));
            assert!(r.is_ok(), "{:?}: {:?}", ad.get_info().backend, r.err());
            provados += 1;
        }
        eprintln!("backends provados: {provados}");
    }
}
