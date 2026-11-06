// SPDX-License-Identifier: LGPL-2.1-or-later
//! Viewport 3D própria (wgpu): renderiza em textura offscreen (MSAA + profundidade) que o egui exibe.
//! Só renderiza de novo quando câmera, cena, estilo ou tamanho mudam.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;
use crate::i18n::tr;

const SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    params: vec4<f32>,     // x = estilo, yz = tamanho (px), w = avanço das arestas no clip (≈ 1 px de profundidade)
    bg_top: vec4<f32>,
    bg_bottom: vec4<f32>,  // w = 1 → degradê
    paper: vec4<f32>,      // cor das faces no estilo "linhas ocultas"
    tex_scale: array<vec4<f32>, 2>,  // tamanho (mm) de uma repetição de cada textura
    qual: vec4<f32>,       // x = 1 → iluminação realista (três pontos, ambiente hemisférico, gama)
    secao: vec4<f32>,      // plano de corte da prévia: xyz = normal (zero = sem corte), w = distância
    mapa: vec4<f32>,       // mapa por pixel: x = tipo de curvatura, y = raio da escala, z = saída mínima (°), w = tampa (°)
    mapa_dir: vec4<f32>,   // direção de abertura da inclinação
    mapa_cor: array<vec4<f32>, 4>,  // paradas da escala do tema
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d_array<f32>;
@group(0) @binding(2) var samp: sampler;

// textura triplanar: projeta a imagem nos três planos e mistura pela normal
fn triplanar(layer: i32, p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let s = max(u.tex_scale[layer / 4][layer % 4], 0.001);
    var w = pow(abs(n), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let a = textureSampleLevel(tex, samp, p.yz / s, layer, 0.0).rgb;
    let b = textureSampleLevel(tex, samp, p.xz / s, layer, 0.0).rgb;
    let c = textureSampleLevel(tex, samp, p.xy / s, layer, 0.0).rgb;
    return a * w.x + b * w.y + c * w.z;
}

// o ponto fica do lado cortado do plano de seção da prévia?
fn cortado(p: vec3<f32>) -> bool {
    return dot(u.secao.xyz, u.secao.xyz) > 0.5 && dot(u.secao.xyz, p) > u.secao.w;
}

// A textura da viewport é exibida pelo egui sem reconversão: as cores do tema são valores de tela (sRGB).
// A iluminação realista soma luz em espaço linear e devolve o resultado em sRGB.
fn to_linear(c: vec3<f32>) -> vec3<f32> { return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2)); }
fn to_display(c: vec3<f32>) -> vec3<f32> { return pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)); }

// luz de três pontos presa à câmera (principal acima à esquerda, preenchimento à direita, contraluz
// fraco atrás), ambiente hemisférico (céu/chão, Z para cima) e brilho Blinn-Phong leve
fn shade_realistic(base_s: vec3<f32>, n: vec3<f32>, v: vec3<f32>, rough: f32, metal: f32) -> vec3<f32> {
    let base = to_linear(base_s);
    var r = cross(vec3<f32>(0.0, 0.0, 1.0), v);
    if (length(r) < 1e-3) { r = vec3<f32>(1.0, 0.0, 0.0); }
    let right = normalize(r);
    let up = cross(v, right);
    let key = normalize(v * 0.55 + up * 0.60 - right * 0.55);
    let fill = normalize(v * 0.70 + right * 0.70 - up * 0.10);
    let rim = normalize(-v * 0.80 + up * 0.50 + right * 0.30);

    let sky = vec3<f32>(0.62, 0.68, 0.80);
    let ground = vec3<f32>(0.20, 0.18, 0.16);
    let amb = mix(ground, sky, n.z * 0.5 + 0.5) * 0.34;
    let kd = max(dot(n, key), 0.0);
    let fd = max(dot(n, fill), 0.0);
    let diffc = base * (1.0 - metal * 0.75);
    var c = diffc * (amb + vec3<f32>(1.0, 0.98, 0.94) * kd * 0.70 + vec3<f32>(0.80, 0.88, 1.0) * fd * 0.24);

    // contraluz: só nas bordas voltadas para longe da câmera
    let edge = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    c = c + diffc * sky * edge * max(dot(n, rim) + 0.3, 0.0) * 0.28;

    let shin = mix(110.0, 10.0, rough);
    let spec = pow(max(dot(n, normalize(key + v)), 0.0), shin) * (1.0 - rough * 0.8);
    c = c + mix(vec3<f32>(1.0), base, metal) * spec * 0.30 * kd;
    c = c + base * metal * amb * 0.5;
    return to_display(c);
}

// Escala do tema na posição `t`: a mesma de `sup_analise::rampa`, agora por pixel. Interpolar a cor
// entre vértices, como antes, desenhava os triângulos na tela: a escala é logarítmica e tem paradas.
fn rampa(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0) * 3.0;
    let k = min(i32(floor(x)), 2);
    return mix(u.mapa_cor[k].rgb, u.mapa_cor[k + 1].rgb, x - f32(k));
}

// Cor da curvatura (`EscalaCurvatura::cor`): as curvaturas principais vêm interpoladas e a escala vale aqui.
fn cor_curvatura(k1: f32, k2: f32) -> vec3<f32> {
    var k = max(abs(k1), abs(k2));
    if (u.mapa.x > 1.5) { k = sqrt(abs(k1 * k2)); } else if (u.mapa.x > 0.5) { k = abs(k1 + k2) * 0.5; }
    return rampa(log2(1.0 + k * u.mapa.y * 15.0) / 4.0);
}

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
    // derivadas antes de qualquer desvio dependente do pixel (exigência de uniformidade do WGSL)
    let g = cross(dpdx(i.wp), dpdy(i.wp));
    if (cortado(i.wp)) { discard; }
    let mode = u.params.x;
    let v = normalize(u.eye.xyz - i.wp);
    var n = normalize(i.n);
    // De que lado se olha quem diz é o triângulo. Perto da silhueta a normal interpolada já aponta para
    // longe do olho num triângulo ainda de frente, e virá-la acendia um reflexo errado na borda.
    let frente = dot(g, v);
    var lado = dot(n, v);
    if (abs(frente) > 1e-20) { lado = dot(n, g) * frente; }
    if (lado < 0.0) { n = -n; }
    let nv = dot(n, v);
    if (nv < 0.0) { n = normalize(n - v * nv); }
    if (mode > 1.5 && mode < 2.5) { return vec4<f32>(u.paper.rgb, 1.0); }

    var base = i.col.rgb;
    if (i.mat.w > 0.5) { base = base * triplanar(i32(i.mat.w + 0.5) - 1, i.wp, normalize(i.n)); }
    let rough = i.mat.x;
    let metal = i.mat.y;
    var alpha = i.mat.z * i.col.a;

    var c: vec3<f32>;
    if (mode > 8.5) {
        // inclinação por pixel: o ângulo de saída vem da normal, e a faixa troca onde a superfície troca
        let ang = degrees(asin(clamp(abs(dot(n, u.mapa_dir.xyz)), 0.0, 1.0)));
        let w = max(fwidth(ang), 1e-3);
        let f = 1.0 - smoothstep(u.mapa.z - w, u.mapa.z + w, ang);
        let f2 = 1.0 - smoothstep(2.0 * u.mapa.z - w, 2.0 * u.mapa.z + w, ang);
        let f3 = 1.0 - smoothstep(u.mapa.w - w, u.mapa.w + w, ang);
        let d = abs(dot(n, v));
        var cor = i.col.rgb;
        if (i.mat.w < -0.5) { cor = rampa((f + f2 + f3) / 3.0); }
        return vec4<f32>(cor * (0.55 + 0.45 * d), 1.0);
    }
    if (mode > 7.5 && mode < 8.5) {
        // isofotas: linhas onde o ângulo entre a normal e a luz é o mesmo; a luz fica presa ao mundo
        let l = normalize(vec3<f32>(0.35, -0.45, 0.82));
        let s = dot(n, l) * 14.0;
        let w = max(fwidth(s), 1e-4);
        let linha = 1.0 - smoothstep(0.0, 1.5 * w + 0.06, abs(fract(s + 0.5) - 0.5));
        let d = abs(dot(n, v));
        let fundo = vec3<f32>(0.82, 0.84, 0.88) * (0.70 + 0.30 * d);
        return vec4<f32>(mix(fundo, vec3<f32>(0.05, 0.06, 0.09), linha), 1.0);
    }
    if (mode > 6.5) {
        // curvatura: a cena traz as curvaturas medidas na superfície (B34, B346) e a cor sai aqui,
        // por pixel; o que vier sem medida (`mat.w` >= 0) mantém a própria cor
        let d = abs(dot(n, v));
        var cor = i.col.rgb;
        if (i.mat.w < -0.5) { cor = cor_curvatura(i.col.x, i.col.y); }
        return vec4<f32>(cor * (0.55 + 0.45 * d), 1.0);
    }
    if (mode > 5.5) {
        // zebra: faixas do reflexo de um ambiente de listras horizontais
        let r = reflect(-v, n);
        let s = r.z * 9.0;
        let w = max(fwidth(s), 1e-4);
        let band = smoothstep(0.5 - w, 0.5 + w, abs(fract(s) - 0.5) * 2.0);
        let d = abs(dot(n, v));
        return vec4<f32>(vec3<f32>(band * 0.92 + 0.04) * (0.75 + 0.25 * d), 1.0);
    }
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
    } else if (u.qual.x > 0.5) {
        c = shade_realistic(base, n, v, rough, metal);
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

// ---------- fantasma: prévia do comando, na cor que a cena mandou. A borda sai mais densa para o
// contorno aparecer mesmo com a malha quase transparente de frente.
@fragment fn fs_ghost(i: MOut) -> @location(0) vec4<f32> {
    if (cortado(i.wp)) { discard; }
    let v = normalize(u.eye.xyz - i.wp);
    let d = abs(dot(normalize(i.n), v));
    let a = i.col.a * (1.0 + 1.2 * pow(1.0 - d, 2.0));
    return vec4<f32>(i.col.rgb * (0.72 + 0.28 * d), clamp(a, 0.0, 0.9));
}

// ---------- linhas
struct LIn { @location(0) pos: vec3<f32>, @location(1) col: vec4<f32> };
struct LOut { @builtin(position) clip: vec4<f32>, @location(0) col: vec4<f32>, @location(1) wp: vec3<f32> };
@vertex fn vs_line(v: LIn) -> LOut {
    var o: LOut;
    var p = u.view_proj * vec4<f32>(v.pos, 1.0);
    p.z = p.z - u.params.w;  // arestas ≈ 1 px à frente das faces coincidentes, em qualquer distância
    o.clip = p; o.col = v.col; o.wp = v.pos;
    return o;
}
// ---------- cópias da montagem: a malha é a do protótipo e a posição vem por cópia (B125)
struct IIn {
    @location(2) m0: vec4<f32>, @location(3) m1: vec4<f32>, @location(4) m2: vec4<f32>,
    @location(5) col: vec4<f32>, @location(6) mat: vec4<f32>, @location(7) aresta: vec4<f32>,
};
fn posto(i: IIn, p: vec4<f32>) -> vec3<f32> { return vec3<f32>(dot(i.m0, p), dot(i.m1, p), dot(i.m2, p)); }
@vertex fn vs_mesh_i(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, i: IIn) -> MOut {
    var o: MOut;
    let w = posto(i, vec4<f32>(pos, 1.0));
    o.clip = u.view_proj * vec4<f32>(w, 1.0);
    o.n = posto(i, vec4<f32>(nrm, 0.0)); o.col = i.col; o.wp = w; o.mat = i.mat;
    return o;
}
@vertex fn vs_line_i(@location(0) pos: vec3<f32>, i: IIn) -> LOut {
    var o: LOut;
    let w = posto(i, vec4<f32>(pos, 1.0));
    var p = u.view_proj * vec4<f32>(w, 1.0);
    p.z = p.z - u.params.w;
    o.clip = p; o.col = i.aresta; o.wp = w;
    return o;
}
@fragment fn fs_line(i: LOut) -> @location(0) vec4<f32> {
    if (cortado(i.wp)) { discard; }
    return i.col;
}

// ---------- linhas grossas e com tipo de linha (cada segmento = 2 triângulos expandidos na tela)
// p: x = lado (-1/+1), y = ponta (0 = a, 1 = b), z = espessura (px), w = tipo (0 contínua, 1 tracejada,
// 2 traço-ponto, 3 pontilhada); q: x = comprimento acumulado até a, y = comprimento do segmento
struct WIn { @location(0) a: vec3<f32>, @location(1) b: vec3<f32>, @location(2) col: vec4<f32>, @location(3) p: vec4<f32>, @location(4) q: vec2<f32> };
struct WOut { @builtin(position) clip: vec4<f32>, @location(0) col: vec4<f32>, @location(1) dist: f32, @location(2) @interpolate(flat) dash: f32 };
@vertex fn vs_wide(v: WIn) -> WOut {
    var o: WOut;
    let ca = u.view_proj * vec4<f32>(v.a, 1.0);
    let cb = u.view_proj * vec4<f32>(v.b, 1.0);
    let size = max(u.params.yz, vec2<f32>(1.0, 1.0));
    let sa = ca.xy / max(ca.w, 1e-6) * size * 0.5;
    let sb = cb.xy / max(cb.w, 1e-6) * size * 0.5;
    var d = sb - sa;
    let len_px = length(d);
    if (len_px < 1e-6) { d = vec2<f32>(1.0, 0.0); } else { d = d / len_px; }
    let n = vec2<f32>(-d.y, d.x);
    var c = select(ca, cb, v.p.y > 0.5);
    let off = n * v.p.x * v.p.z * 0.5 / (size * 0.5);
    c = vec4<f32>(c.xy + off * c.w, c.z - 2.0 * u.params.w, c.w);
    o.clip = c;
    o.col = v.col;
    // distância em px ao longo da polilinha (para o tracejado)
    let px_per_unit = len_px / max(v.q.y, 1e-6);
    o.dist = (v.q.x + v.p.y * v.q.y) * px_per_unit;
    o.dash = v.p.w;
    return o;
}
@fragment fn fs_wide(i: WOut) -> @location(0) vec4<f32> {
    if (i.dash > 0.5) {
        var period = 13.0; var on = 8.0;
        if (i.dash > 2.5) { period = 6.0; on = 2.0; }
        let t = i.dist - floor(i.dist / select(period, 22.0, i.dash > 1.5 && i.dash < 2.5)) * select(period, 22.0, i.dash > 1.5 && i.dash < 2.5);
        if (i.dash > 1.5 && i.dash < 2.5) {
            // traço-ponto: 12 traço, 4 vazio, 2 ponto, 4 vazio
            if ((t > 12.0 && t < 16.0) || t > 18.0) { discard; }
        } else if (t > on) {
            discard;
        }
    }
    return i.col;
}
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
    /// análise: listras de reflexo
    Zebra,
    /// análise: mapa de curvatura
    Curvatura,
    /// análise: ângulo de saída de cada face para a direção de abertura (B180, B197)
    Inclinacao,
    /// análise: isofotas, linhas de mesmo ângulo com a luz (B180)
    Isofotas,
    /// análise de fabricação: o painel de DFM varre a peça e a cena realça as faces (B112, B113, B197)
    Dfm,
}

impl DisplayMode {
    pub const ALL: [DisplayMode; 11] = [
        DisplayMode::SombreadoArestas,
        DisplayMode::Sombreado,
        DisplayMode::LinhasOcultas,
        DisplayMode::Arame,
        DisplayMode::RaioX,
        DisplayMode::SemiRenderizado,
        DisplayMode::Zebra,
        DisplayMode::Curvatura,
        DisplayMode::Inclinacao,
        DisplayMode::Isofotas,
        DisplayMode::Dfm,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::SombreadoArestas => tr("Sombreado com arestas"),
            DisplayMode::Sombreado => tr("Sombreado (sem arestas)"),
            DisplayMode::LinhasOcultas => tr("Linhas ocultas removidas"),
            DisplayMode::Arame => tr("Arame"),
            DisplayMode::RaioX => tr("Raio-X"),
            DisplayMode::SemiRenderizado => tr("Semi-renderizado"),
            DisplayMode::Zebra => tr("Análise: zebra"),
            DisplayMode::Curvatura => tr("Análise: curvatura"),
            DisplayMode::Inclinacao => tr("Análise: inclinação"),
            DisplayMode::Isofotas => tr("Análise: isofotas"),
            DisplayMode::Dfm => tr("Análise: fabricação"),
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
            DisplayMode::Zebra => 6.,
            // o DFM sombreia como o modo comum
            DisplayMode::Curvatura => 7.,
            DisplayMode::Inclinacao => 9.,
            DisplayMode::Isofotas => 8.,
            DisplayMode::Dfm => 0.,
        }
    }
}

/// Anti-serrilhamento (MSAA) pedido nas configurações.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Msaa {
    Desligado,
    X2,
    #[default]
    X4,
    X8,
}

impl Msaa {
    pub const ALL: [Msaa; 4] = [Msaa::Desligado, Msaa::X2, Msaa::X4, Msaa::X8];
    pub fn amostras(self) -> u32 {
        match self {
            Msaa::Desligado => 1,
            Msaa::X2 => 2,
            Msaa::X4 => 4,
            Msaa::X8 => 8,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Msaa::Desligado => tr("Desligado"),
            Msaa::X2 => "2×",
            Msaa::X4 => "4×",
            Msaa::X8 => "8×",
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
    /// amostras de anti-serrilhamento pedidas (1 = desligado); a viewport cai no maior valor que a placa aceita
    pub msaa: u32,
    /// "Simplificar ao girar" ligado: enquanto `navegando`, desenha sem anti-serrilhamento
    pub simplificar: bool,
    pub navegando: bool,
    /// modo leve (B236): a montagem aceita um erro maior antes de pedir a malha fina
    pub leve: bool,
    /// iluminação realista no sombreado (três pontos, ambiente hemisférico, gama)
    pub realista: bool,
    /// plano de corte da prévia: normal (unitária) e distância; o lado positivo some
    pub secao: Option<[f32; 4]>,
    /// escala dos mapas calculados por pixel (B346)
    pub mapa: Option<MapaGpu>,
}

/// O que o shader precisa para pintar o mapa de curvatura ou de inclinação por pixel.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MapaGpu {
    /// 0 máxima, 1 média, 2 gaussiana
    pub tipo: f32,
    /// raio que recebe a última cor, em mm
    pub raio: f32,
    /// saída mínima e início da tampa, em graus
    pub minimo: f32,
    pub tampa: f32,
    /// direção de abertura
    pub dir: [f32; 3],
    pub paradas: [[f32; 3]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub col: [f32; 4],
    /// rugosidade, metalicidade, opacidade, textura (1 em diante) ou `MAPA_POR_PIXEL`
    pub mat: [f32; 4],
}

/// Em `MeshVertex::mat[3]`: o vértice leva as curvaturas em `col` e a cor sai no shader.
pub const MAPA_POR_PIXEL: f32 = -1.;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub col: [f32; 4],
}

/// Vértice das linhas grossas (6 por segmento).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WideVertex {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub col: [f32; 4],
    pub p: [f32; 4],
    pub q: [f32; 2],
}

/// Acrescenta uma polilinha grossa (espessura em px; tipo 0 contínua, 1 tracejada, 2 traço-ponto, 3 pontilhada).
pub fn push_wide(out: &mut Vec<WideVertex>, pts: &[[f32; 3]], col: [f32; 4], width: f32, dash: u8) {
    let mut acc = 0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        for (side, end) in [(-1., 0.), (1., 0.), (1., 1.), (-1., 0.), (1., 1.), (-1., 1.)] {
            out.push(WideVertex { a, b, col, p: [side, end, width, dash as f32], q: [acc, len] });
        }
        acc += len;
    }
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
    tex_scale: [[f32; 4]; 2],
    qual: [f32; 4],
    secao: [f32; 4],
    mapa: [f32; 4],
    mapa_dir: [f32; 4],
    mapa_cor: [[f32; 4]; 4],
}

/// Camadas do array de texturas (cada imagem é reduzida para TEX × TEX).
pub const TEX_LAYERS: u32 = 8;
const TEX: u32 = 512;

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
    /// linhas grossas / com tipo de linha (curvas de referência), com profundidade
    pub wide: Vec<WideVertex>,
    /// sombra translúcida dos perfis fechados de sketch (triângulos sem índice)
    pub fill: Vec<MeshVertex>,
    /// prévia do comando aberto (triângulos sem índice): translúcida, nunca entra na seleção
    pub ghost: Vec<MeshVertex>,
    pub bounds: Option<(Vec3, Vec3)>,
    /// cópias da montagem: a malha sobe uma vez por protótipo, não entra em `mesh` nem em `edges` (B125)
    pub lotes: Vec<crate::instancias::Lote>,
    /// as cópias desenham também as arestas de tangência
    pub tangentes: bool,
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
    /// Pixels por milímetro a `distancia` do olho, numa vista de `altura_px` de altura.
    pub fn px_por_mm(&self, distancia: f32, altura_px: f32) -> f32 {
        let d = if self.ortho { self.dist } else { distancia.max(self.dist * 0.01) };
        altura_px / (2. * d * (FOV / 2.).tan())
    }
    /// Quanto as arestas são puxadas para a câmera (em unidades de clip): o equivalente a
    /// `ARESTA_PX` pixels de profundidade na distância de cada aresta. Na perspectiva isso dá um valor
    /// constante no clip (perto · tamanho do pixel por unidade de distância); um avanço proporcional a
    /// `w` crescia com a distância e deixava aparecer as arestas de trás perto dos cantos.
    pub fn line_bias(&self, height_px: f32) -> f32 {
        const ARESTA_PX: f32 = 1.5;
        let h = height_px.max(1.);
        let px = 2. * (FOV / 2.).tan() / h * ARESTA_PX;
        if self.ortho {
            // profundidade linear em [-50·dist, 50·dist]; pixel = 2·dist·tan(FOV/2)/h
            px * self.dist / (100. * self.dist)
        } else {
            self.dist * 0.01 * px
        }
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
    /// Projeta um ponto do mundo para a tela.
    pub fn project(&self, rect: egui::Rect, p: Vec3) -> Option<egui::Pos2> {
        let c = self.view_proj(rect.width() / rect.height()) * p.extend(1.0);
        if c.w <= 0. {
            return None;
        }
        let n = c.truncate() / c.w;
        Some(egui::pos2(rect.left() + (n.x + 1.) / 2. * rect.width(), rect.top() + (1. - n.y) / 2. * rect.height()))
    }
    /// Raio (origem, direção) que passa pelo ponto da tela.
    pub fn ray(&self, rect: egui::Rect, pos: egui::Pos2) -> (Vec3, Vec3) {
        let inv = self.view_proj(rect.width() / rect.height()).inverse();
        let nx = (pos.x - rect.left()) / rect.width() * 2. - 1.;
        let ny = 1. - (pos.y - rect.top()) / rect.height() * 2.;
        let a = inv.project_point3(Vec3::new(nx, ny, 0.));
        let b = inv.project_point3(Vec3::new(nx, ny, 1.));
        (a, (b - a).normalize_or_zero())
    }
    /// Olha de frente para um plano (olho do lado da normal).
    pub fn look_along(&mut self, normal: Vec3, center: Vec3) {
        let n = normal.normalize();
        self.pitch = n.z.clamp(-1., 1.).asin().clamp(-1.5706, 1.5706);
        if n.x.abs() + n.y.abs() > 1e-4 {
            self.yaw = n.y.atan2(n.x);
        } else {
            // plano horizontal: X para a direita, Y para cima na tela
            self.yaw = -std::f32::consts::FRAC_PI_2 * n.z.signum();
        }
        self.target = center;
    }
}

/// Cor e profundidade de uma contagem de amostras (com 1 amostra a cor é a própria `resolve`).
struct MsTarget {
    samples: u32,
    color: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
}

struct Targets {
    size: [u32; 2],
    ms: Vec<MsTarget>,
    resolve: wgpu::Texture,
    tex_id: egui::TextureId,
}

/// Os pipelines de uma contagem de amostras.
struct Pipes {
    samples: u32,
    bg: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    xray: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    overlay: wgpu::RenderPipeline,
    wide: wgpu::RenderPipeline,
    /// fantasma em dois passos: só profundidade, depois a cor. Assim cada pixel mistura uma camada
    /// só, a mais próxima, sem depender da ordem dos triângulos.
    ghost_z: wgpu::RenderPipeline,
    ghost: wgpu::RenderPipeline,
    /// os mesmos desenhos para as cópias da montagem
    mesh_i: wgpu::RenderPipeline,
    xray_i: wgpu::RenderPipeline,
    line_i: wgpu::RenderPipeline,
    overlay_i: wgpu::RenderPipeline,
}

type Buf = Option<(wgpu::Buffer, u32)>;

pub struct Viewport {
    module: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    /// pipelines por contagem de amostras (só as que a qualidade atual pode usar)
    pipes: Vec<Pipes>,
    /// contagens de amostras que a placa aceita para cor e profundidade (sempre inclui 1)
    suportadas: Vec<u32>,
    wb: Buf,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    vb: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    eb: Buf,
    lb: Buf,
    fb: Option<(wgpu::Buffer, u32)>,
    ob: Buf,
    gb: Buf,
    targets: Option<Targets>,
    pub camera: Camera,
    textures: wgpu::Texture,
    tex_scale: [[f32; 4]; 2],
    /// transição suave da câmera (de, para, início)
    anim: Option<(Camera, Camera, std::time::Instant)>,
    last: Option<(Camera, [u32; 2], u64, ViewStyle)>,
    scene_gen: u64,
    pub bounds: Option<(Vec3, Vec3)>,
    /// a última cena passou do maior buffer que a placa aceita e ficou sem as faces
    pub cena_grande: bool,
    /// cópias da montagem na placa (B125)
    pub inst: crate::instancias::Instanciador,
    /// último movimento de câmera (órbita, pan, zoom, animação) para "Simplificar ao girar"
    ultimo_giro: Option<std::time::Instant>,
    /// transições de câmera ligadas (desligadas no modo leve: a vista salta direto)
    pub animar: bool,
}

const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Quanto tempo depois do último movimento a câmera ainda conta como "navegando".
pub const PAUSA_NAVEGACAO: std::time::Duration = std::time::Duration::from_millis(220);

/// Contagens de amostras (1, 2, 4, 8) que o dispositivo aceita para a cor e a profundidade.
/// Só 1 e 4 são garantidas; 2 e 8 dependem da feature de formatos específicos do adaptador.
fn amostras_suportadas(rs: &egui_wgpu::RenderState) -> Vec<u32> {
    let extra = rs.device.features().contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
    let aceita = |fmt: wgpu::TextureFormat, n: u32| n == 1 || (rs.adapter.get_texture_format_features(fmt).flags.sample_count_supported(n) && (extra || n == 4));
    [1u32, 2, 4, 8].into_iter().filter(|n| aceita(FMT, *n) && aceita(DEPTH, *n)).collect()
}

/// Cor da interface (sRGB 0–255) para o shader. A textura é exibida pelo egui sem reconversão,
/// então os valores vão diretos, como as cores das faces.
fn lin(c: [u8; 3]) -> [f32; 4] {
    let f = |x: u8| x as f32 / 255.;
    [f(c[0]), f(c[1]), f(c[2]), 1.]
}

/// Monta os pipelines para uma contagem de amostras.
fn make_pipes(dev: &wgpu::Device, module: &wgpu::ShaderModule, layout: &wgpu::PipelineLayout, samples: u32) -> Pipes {
    let target = [Some(wgpu::ColorTargetState { format: FMT, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })];
    let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4];
    let line_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
    let wide_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x4, 4 => Float32x2];
    use wgpu::CompareFunction as C;
    use wgpu::PrimitiveTopology as T;
    let pipe = |vs: &str, fs: &str, attrs: &[wgpu::VertexAttribute], stride: usize, topo: T, cmp, write| {
        let buffers = [Some(wgpu::VertexBufferLayout { array_stride: stride as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: attrs })];
        dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(vs),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some(vs),
                buffers: if stride == 0 { &[] } else { &buffers },
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState { module, entry_point: Some(fs), targets: &target, compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState { topology: if topo == T::LineStrip { T::TriangleList } else { topo }, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(write),
                depth_compare: Some(cmp),
                stencil: Default::default(),
                // faces um pouco para trás (polygon offset) só pelo termo constante: o termo de
                // inclinação empurrava faces vistas de lado tanto que arestas escondidas atrás delas
                // apareciam perto dos cantos (a frente das arestas vem de `Camera::line_bias`)
                bias: if topo == T::TriangleList && stride != std::mem::size_of::<WideVertex>() {
                    wgpu::DepthBiasState { constant: 2, slope_scale: 0.0, clamp: 0.0 }
                } else {
                    Default::default()
                },
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
            cache: None,
        })
    };
    let ms = std::mem::size_of::<MeshVertex>();
    let ls = std::mem::size_of::<LineVertex>();
    // cópias: posição e normal em buffers próprios, direto da malha, e uma entrada por cópia
    let inst_attrs = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];
    let pos_attr = wgpu::vertex_attr_array![0 => Float32x3];
    let nrm_attr = wgpu::vertex_attr_array![1 => Float32x3];
    let copia = |vs: &str, fs: &str, topo: T, cmp, write, faces: bool| {
        let vb = |attributes| wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes };
        let ib = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<crate::instancias::InstanciaGpu>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &inst_attrs };
        let com_normal = [Some(vb(&pos_attr)), Some(vb(&nrm_attr)), Some(ib.clone())];
        let so_posicao = [Some(vb(&pos_attr)), Some(ib)];
        dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(vs),
            layout: Some(layout),
            vertex: wgpu::VertexState { module, entry_point: Some(vs), buffers: if faces { &com_normal } else { &so_posicao }, compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module, entry_point: Some(fs), targets: &target, compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState { topology: topo, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(write),
                depth_compare: Some(cmp),
                stencil: Default::default(),
                bias: if faces { wgpu::DepthBiasState { constant: 2, slope_scale: 0.0, clamp: 0.0 } } else { Default::default() },
            }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
            cache: None,
        })
    };
    // sem o recuo das faces: o fantasma coincidente com a peça fica sempre por cima dela
    let fantasma = |writes: wgpu::ColorWrites, cmp, write| {
        let buffers = [Some(wgpu::VertexBufferLayout { array_stride: ms as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &mesh_attrs })];
        let alvo = [Some(wgpu::ColorTargetState { format: FMT, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: writes })];
        dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fantasma"),
            layout: Some(layout),
            vertex: wgpu::VertexState { module, entry_point: Some("vs_mesh"), buffers: &buffers, compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module, entry_point: Some("fs_ghost"), targets: &alvo, compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState { topology: T::TriangleList, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(write), depth_compare: Some(cmp), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
            cache: None,
        })
    };
    Pipes {
        samples,
        bg: pipe("vs_bg", "fs_bg", &[], 0, T::TriangleList, C::Always, false),
        mesh: pipe("vs_mesh", "fs_mesh", &mesh_attrs, ms, T::TriangleList, C::Less, true),
        xray: pipe("vs_mesh", "fs_mesh", &mesh_attrs, ms, T::TriangleList, C::Less, false),
        line: pipe("vs_line", "fs_line", &line_attrs, ls, T::LineList, C::LessEqual, false),
        overlay: pipe("vs_line", "fs_line", &line_attrs, ls, T::LineList, C::Always, false),
        wide: pipe("vs_wide", "fs_wide", &wide_attrs, std::mem::size_of::<WideVertex>(), T::LineStrip, C::LessEqual, false),
        ghost_z: fantasma(wgpu::ColorWrites::empty(), C::Less, true),
        ghost: fantasma(wgpu::ColorWrites::ALL, C::LessEqual, false),
        mesh_i: copia("vs_mesh_i", "fs_mesh", T::TriangleList, C::Less, true, true),
        xray_i: copia("vs_mesh_i", "fs_mesh", T::TriangleList, C::Less, false, true),
        line_i: copia("vs_line_i", "fs_line", T::LineList, C::LessEqual, false, false),
        overlay_i: copia("vs_line_i", "fs_line", T::LineList, C::Always, false, false),
    }
}

impl Viewport {
    /// A câmera mudou (órbita, pan, zoom, animação): marca o instante para "Simplificar ao girar".
    pub fn navegou(&mut self) {
        self.ultimo_giro = Some(std::time::Instant::now());
    }

    /// A câmera se mexeu há pouco? (`PAUSA_NAVEGACAO`)
    pub fn navegando(&self) -> bool {
        self.ultimo_giro.is_some_and(|t| t.elapsed() < PAUSA_NAVEGACAO)
    }

    /// Altura do último quadro, em pixels da tela.
    pub fn altura_px(&self) -> Option<u32> {
        self.targets.as_ref().map(|t| t.size[1])
    }

    /// A câmera ainda vai andar sozinha?
    pub fn animando(&self) -> bool {
        self.anim.is_some()
    }

    /// Maior contagem de amostras aceita pela placa que não passe do pedido.
    pub fn amostras_efetivas(&self, pedido: u32) -> u32 {
        self.suportadas.iter().copied().filter(|n| *n <= pedido).max().unwrap_or(1)
    }

    /// A câmera mudou por um comando (vista, ajustar, alinhar): em vez de saltar, anima a partir de `before`.
    pub fn animate_from(&mut self, before: Camera) {
        let mut to = self.camera;
        // menor caminho no giro horizontal
        let tau = std::f32::consts::TAU;
        let mut dy = (to.yaw - before.yaw).rem_euclid(tau);
        if dy > std::f32::consts::PI {
            dy -= tau;
        }
        to.yaw = before.yaw + dy;
        let moved = (to.target - before.target).length() > 1e-3 * before.dist.max(1.)
            || dy.abs() > 1e-3
            || (to.pitch - before.pitch).abs() > 1e-3
            || (to.dist / before.dist.max(1e-6) - 1.).abs() > 1e-3;
        if !moved {
            return;
        }
        if !self.animar {
            return;
        }
        let from = Camera { ortho: to.ortho, ..before };
        self.anim = Some((from, to, std::time::Instant::now()));
        self.camera = from;
    }

    /// Avança a animação; devolve true enquanto ela estiver em andamento.
    pub fn step_anim(&mut self) -> bool {
        let Some((a, b, t0)) = self.anim else { return false };
        let t = (t0.elapsed().as_secs_f32() / 0.45).min(1.);
        // desacelera no fim (ease-out cúbico)
        let e = 1. - (1. - t).powi(3);
        self.camera = Camera {
            target: a.target.lerp(b.target, e),
            yaw: a.yaw + (b.yaw - a.yaw) * e,
            pitch: a.pitch + (b.pitch - a.pitch) * e,
            dist: a.dist * (b.dist / a.dist.max(1e-6)).powf(e),
            ortho: b.ortho,
        };
        if t >= 1. {
            self.camera = b;
            self.anim = None;
        }
        true
    }

    /// O usuário mexeu na câmera: a animação para onde está.
    pub fn stop_anim(&mut self) {
        self.anim = None;
    }

    pub fn new(rs: &egui_wgpu::RenderState) -> Self {
        let dev = &rs.device;
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fenkra"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        // texturas: array de camadas brancas até o usuário escolher imagens
        let textures = dev.create_texture(&wgpu::TextureDescriptor {
            label: Some("texturas"),
            size: wgpu::Extent3d { width: TEX, height: TEX, depth_or_array_layers: TEX_LAYERS },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let white = vec![255u8; (TEX * TEX * 4) as usize];
        for l in 0..TEX_LAYERS {
            write_layer(&rs.queue, &textures, l, &white);
        }
        let tex_view = textures.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], ..Default::default() });
        Viewport {
            module,
            layout,
            pipes: vec![],
            suportadas: amostras_suportadas(rs),
            ultimo_giro: None,
            wb: None,
            ubuf,
            bind,
            textures,
            tex_scale: [[100.; 4]; 2],
            vb: None,
            eb: None,
            lb: None,
            fb: None,
            ob: None,
            gb: None,
            targets: None,
            camera: Camera::default(),
            anim: None,
            animar: true,
            last: None,
            scene_gen: 0,
            cena_grande: false,
            inst: Default::default(),
            bounds: None,
        }
    }

    /// Carrega as imagens de textura (RGBA já em TEX × TEX) e o tamanho de repetição de cada uma.
    pub fn set_textures(&mut self, rs: &egui_wgpu::RenderState, imgs: &[(Vec<u8>, f32)]) {
        let white = vec![255u8; (TEX * TEX * 4) as usize];
        for l in 0..TEX_LAYERS {
            let (data, scale) = imgs.get(l as usize).map(|(d, s)| (d.as_slice(), *s)).unwrap_or((&white, 100.));
            let data = if data.len() == (TEX * TEX * 4) as usize { data } else { &white };
            write_layer(&rs.queue, &self.textures, l, data);
            self.tex_scale[l as usize / 4][l as usize % 4] = scale;
        }
        self.last = None;
    }

    pub fn set_scene(&mut self, rs: &egui_wgpu::RenderState, s: &Scene) {
        let dev = &rs.device;
        let mk = |data: &[u8], usage| dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: data, usage });
        let lines = |v: &Vec<LineVertex>| (!v.is_empty()).then(|| (mk(bytemuck::cast_slice(v), wgpu::BufferUsages::VERTEX), v.len() as u32));
        // além do que a placa aceita, criar o buffer derruba o programa: a cena fica sem as faces e avisa
        let cabe = (std::mem::size_of_val(s.mesh.as_slice()) as u64) <= dev.limits().max_buffer_size;
        self.cena_grande = !cabe || self.inst.grande_demais;
        self.inst.definir(s.lotes.clone(), s.tangentes);
        self.vb = (!s.indices.is_empty() && cabe).then(|| {
            (
                mk(bytemuck::cast_slice(&s.mesh), wgpu::BufferUsages::VERTEX),
                mk(bytemuck::cast_slice(&s.indices), wgpu::BufferUsages::INDEX),
                s.indices.len() as u32,
            )
        });
        self.eb = lines(&s.edges);
        self.fb = (!s.fill.is_empty()).then(|| (mk(bytemuck::cast_slice(&s.fill), wgpu::BufferUsages::VERTEX), s.fill.len() as u32));
        self.lb = lines(&s.lines);
        self.ob = lines(&s.overlay);
        self.gb = (!s.ghost.is_empty()).then(|| (mk(bytemuck::cast_slice(&s.ghost), wgpu::BufferUsages::VERTEX), s.ghost.len() as u32));
        self.wb = (!s.wide.is_empty()).then(|| (mk(bytemuck::cast_slice(&s.wide), wgpu::BufferUsages::VERTEX), s.wide.len() as u32));
        self.bounds = s.bounds;
        self.scene_gen += 1;
    }

    /// Garante pipelines e alvos de cor/profundidade para cada contagem de `usar` (e descarta as outras),
    /// no tamanho pedido. Trocar o anti-serrilhamento recria só o que mudou.
    fn ensure_targets(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2], usar: &[u32]) {
        let dev = &rs.device;
        self.pipes.retain(|p| usar.contains(&p.samples));
        for &n in usar {
            if !self.pipes.iter().any(|p| p.samples == n) {
                self.pipes.push(make_pipes(dev, &self.module, &self.layout, n));
            }
        }
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
        let make_ms = |n: u32| MsTarget {
            samples: n,
            color: (n > 1).then(|| tex(FMT, n, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default())),
            depth: tex(DEPTH, n, wgpu::TextureUsages::RENDER_ATTACHMENT).create_view(&Default::default()),
        };
        if let Some(t) = self.targets.as_mut().filter(|t| t.size == size) {
            t.ms.retain(|m| usar.contains(&m.samples));
            for &n in usar {
                if !t.ms.iter().any(|m| m.samples == n) {
                    t.ms.push(make_ms(n));
                    self.last = None;
                }
            }
            return;
        }
        let ms = usar.iter().map(|n| make_ms(*n)).collect();
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
        self.targets = Some(Targets { size, ms, resolve, tex_id });
        self.last = None;
    }

    /// Renderiza (se necessário) e devolve a textura para exibir.
    pub fn render(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2], style: ViewStyle) -> egui::TextureId {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        // anti-serrilhamento de repouso (o que a placa aceita) e o de navegação (sem, se pedido)
        let repouso = self.amostras_efetivas(style.msaa);
        let usar: Vec<u32> = if style.simplificar && repouso > 1 { vec![repouso, 1] } else { vec![repouso] };
        let amostras = if style.simplificar && style.navegando { 1 } else { repouso };
        self.ensure_targets(rs, size, &usar);
        let key = (self.camera, size, self.scene_gen, style);
        if self.last == Some(key) && !self.inst.subindo {
            return self.targets.as_ref().unwrap().tex_id;
        }
        if self.inst.tem_lotes() {
            let limiar = fenkra_montagem::lod::LIMIAR_PX * if style.simplificar && style.navegando { 4. } else { 1. } * if style.leve { 2. } else { 1. };
            crate::medir_quadros::fase("preparar", || self.inst.preparar(&rs.device, &rs.queue, &crate::instancias::Quadro { camera: self.camera, tamanho: size, limiar_px: limiar }));
        }
        let t = self.targets.as_ref().unwrap();
        let vp = self.camera.view_proj(size[0] as f32 / size[1] as f32);
        let e = self.camera.eye();
        let mut bottom = lin(style.bg_bottom);
        bottom[3] = if style.gradient { 1. } else { 0. };
        let u = Uniforms {
            view_proj: vp.to_cols_array_2d(),
            eye: [e.x, e.y, e.z, 1.],
            params: [style.mode.code(), size[0] as f32, size[1] as f32, self.camera.line_bias(size[1] as f32)],
            bg_top: lin(style.bg_top),
            bg_bottom: bottom,
            paper: [0.96, 0.96, 0.95, 1.],
            tex_scale: self.tex_scale,
            qual: [if style.realista { 1. } else { 0. }, 0., 0., 0.],
            secao: style.secao.unwrap_or([0.; 4]),
            mapa: style.mapa.map_or([0.; 4], |m| [m.tipo, m.raio, m.minimo, m.tampa]),
            mapa_dir: style.mapa.map_or([0., 0., 1., 0.], |m| [m.dir[0], m.dir[1], m.dir[2], 0.]),
            mapa_cor: style.mapa.map_or([[0.; 4]; 4], |m| m.paradas.map(|c| [c[0], c[1], c[2], 1.])),
        };
        rs.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));

        let (Some(ms), Some(p)) = (t.ms.iter().find(|m| m.samples == amostras), self.pipes.iter().find(|p| p.samples == amostras)) else { return t.tex_id };
        let mut enc = rs.device.create_command_encoder(&Default::default());
        {
            let view = t.resolve.create_view(&Default::default());
            // com anti-serrilhamento desenha na textura multiamostrada e resolve na `resolve`;
            // sem ele (ou ao simplificar) desenha direto nela
            let (alvo, resolve, store) = match &ms.color {
                Some(c) => (c, Some(&view), wgpu::StoreOp::Discard),
                None => (&view, None, wgpu::StoreOp::Store),
            };
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: alvo,
                    resolve_target: resolve,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &ms.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_pipeline(&p.bg);
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
                pass.set_pipeline(if m == DisplayMode::RaioX { &p.xray } else { &p.mesh });
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*n, 0, 0..1);
            }
            if faces {
                pass.set_pipeline(if m == DisplayMode::RaioX { &p.xray_i } else { &p.mesh_i });
                self.inst.faces(&mut pass);
            }
            if let (false, Some((fb, n))) = (m == DisplayMode::LinhasOcultas, &self.fb) {
                pass.set_pipeline(&p.xray);
                pass.set_vertex_buffer(0, fb.slice(..));
                pass.draw(0..*n, 0..1);
            }
            let mut lines = |b: &Buf, p: &wgpu::RenderPipeline| {
                if let Some((buf, n)) = b {
                    pass.set_pipeline(p);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..*n, 0..1);
                }
            };
            if edges {
                lines(&self.eb, if edges_always { &p.overlay } else { &p.line });
            }
            lines(&self.lb, &p.line);
            lines(&self.wb, &p.wide);
            if edges {
                pass.set_pipeline(if edges_always { &p.overlay_i } else { &p.line_i });
                self.inst.arestas(&mut pass);
            }
            // o fantasma vem depois de tudo que é opaco e antes das marcas sempre visíveis
            if let Some((gb, n)) = &self.gb {
                pass.set_vertex_buffer(0, gb.slice(..));
                pass.set_pipeline(&p.ghost_z);
                pass.draw(0..*n, 0..1);
                pass.set_pipeline(&p.ghost);
                pass.draw(0..*n, 0..1);
            }
            if let Some((ob, n)) = &self.ob {
                pass.set_pipeline(&p.overlay);
                pass.set_vertex_buffer(0, ob.slice(..));
                pass.draw(0..*n, 0..1);
            }
        }
        rs.queue.submit([enc.finish()]);
        // medição: o tempo da placa só aparece esperando ela terminar
        if crate::medir_quadros::ativo() {
            crate::medir_quadros::fase("placa", || {
                let _ = rs.device.poll(wgpu::PollType::wait_indefinitely());
            });
        }
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

fn write_layer(q: &wgpu::Queue, t: &wgpu::Texture, layer: u32, rgba: &[u8]) {
    q.write_texture(
        wgpu::TexelCopyTextureInfo { texture: t, mip_level: 0, origin: wgpu::Origin3d { x: 0, y: 0, z: layer }, aspect: wgpu::TextureAspect::All },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(TEX * 4), rows_per_image: Some(TEX) },
        wgpu::Extent3d { width: TEX, height: TEX, depth_or_array_layers: 1 },
    );
}

/// Lê uma imagem e reduz para o tamanho das camadas de textura (RGBA).
pub fn load_texture(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    Ok(image::imageops::resize(&img, TEX, TEX, image::imageops::FilterType::Triangle).into_raw())
}
