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
    mapa: vec4<f32>,       // mapa por pixel: x = tipo de curvatura, y = raio da escala, z = saída mínima (°), w = tampa (°); zebra e isofotas: z = faixas, w = estilo
    mapa_dir: vec4<f32>,   // direção de abertura da inclinação; eixo das listras da zebra; luz das isofotas
    mapa_cor: array<vec4<f32>, 4>,  // paradas da escala do tema
    // B139 (ver `render_efeitos.rs`)
    luz_vp: mat4x4<f32>,   // projeção da luz principal para o mapa de sombra
    luz: vec4<f32>,        // xyz = direção para a luz principal, w = recuo de profundidade da sombra
    efeitos: vec4<f32>,    // x = sombra ligada, y = pontos da oclusão (0 = desligada), z = PBR, w = toques da sombra
    efeitos2: vec4<f32>,   // x = texel do mapa de sombra, y = raio da oclusão (mm), w = recuo pela normal (mm)
    prof: vec4<f32>,       // x = perto, y = longe, z = 1 ortográfica
    // B182: cena de apresentação (ver `render_efeitos::UniformeCena`)
    inv_vp: mat4x4<f32>,
    cena: vec4<f32>,       // x = imagem panorâmica, y = giro (rad), z = exposição, w = luz de ambiente
    cena_media: vec4<f32>, // cor média da imagem panorâmica
    luz_cor: vec4<f32>,    // rgb = cor × intensidade da luz principal; w = 1 usa a direção de `luz`
    chao: vec4<f32>,       // xy = centro, z = altura, w = meio lado
    chao2: vec4<f32>,      // x = ligado, y = reflexo, z = sombra no chão, w = faces refletem a imagem
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d_array<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var pano: texture_2d<f32>;

// B182: imagem panorâmica equirretangular, Z para cima, girada em torno de Z
fn ceu(d: vec3<f32>) -> vec3<f32> {
    let a = atan2(d.y, d.x) + u.cena.y;
    let uv = vec2<f32>(fract(a / 6.2831853 + 0.5), acos(clamp(d.z, -1.0, 1.0)) / 3.14159265);
    return textureSampleLevel(pano, samp, uv, 0.0).rgb * u.cena.z;
}
// céu do ambiente hemisférico: com imagem panorâmica, a cor média dela (em espaço linear)
fn ceu_medio(padrao: vec3<f32>) -> vec3<f32> {
    if (u.cena.x > 0.5) { return pow(max(u.cena_media.rgb, vec3<f32>(0.0)), vec3<f32>(2.2)) * 1.3; }
    return padrao;
}

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

// B139: sombra e oclusão. No módulo base são funções que devolvem 1; no módulo dos efeitos leem o
// mapa de sombra e a profundidade da câmera (grupo 1).
//@EFEITOS@

// A textura da viewport é exibida pelo egui sem reconversão: as cores do tema são valores de tela (sRGB).
// A iluminação realista soma luz em espaço linear e devolve o resultado em sRGB.
fn to_linear(c: vec3<f32>) -> vec3<f32> { return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2)); }
fn to_display(c: vec3<f32>) -> vec3<f32> { return pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)); }

// luz de três pontos presa à câmera (principal acima à esquerda, preenchimento à direita, contraluz
// fraco atrás), ambiente hemisférico (céu/chão, Z para cima) e brilho Blinn-Phong leve
// luz principal: com a sombra ligada vem do uniforme, a mesma direção do mapa de sombra
fn luz_principal(v: vec3<f32>, right: vec3<f32>, up: vec3<f32>) -> vec3<f32> {
    if (u.efeitos.x > 0.5 || u.luz_cor.w > 0.5) { return normalize(u.luz.xyz); }
    return normalize(v * 0.55 + up * 0.60 - right * 0.55);
}

fn shade_realistic(base_s: vec3<f32>, n: vec3<f32>, v: vec3<f32>, rough: f32, metal: f32, sh: f32, ao: f32) -> vec3<f32> {
    let base = to_linear(base_s);
    var r = cross(vec3<f32>(0.0, 0.0, 1.0), v);
    if (length(r) < 1e-3) { r = vec3<f32>(1.0, 0.0, 0.0); }
    let right = normalize(r);
    let up = cross(v, right);
    let key = luz_principal(v, right, up);
    let fill = normalize(v * 0.70 + right * 0.70 - up * 0.10);
    let rim = normalize(-v * 0.80 + up * 0.50 + right * 0.30);

    let sky = ceu_medio(vec3<f32>(0.62, 0.68, 0.80));
    let ground = select(vec3<f32>(0.20, 0.18, 0.16), sky * 0.45, u.cena.x > 0.5);
    let amb = mix(ground, sky, n.z * 0.5 + 0.5) * 0.34 * ao * u.cena.w;
    let kd = max(dot(n, key), 0.0) * sh;
    let fd = max(dot(n, fill), 0.0) * mix(1.0, ao, 0.5);
    let diffc = base * (1.0 - metal * 0.75);
    var c = diffc * (amb + vec3<f32>(1.0, 0.98, 0.94) * u.luz_cor.rgb * kd * 0.70 + vec3<f32>(0.80, 0.88, 1.0) * fd * 0.24);

    // contraluz: só nas bordas voltadas para longe da câmera
    let edge = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    c = c + diffc * sky * edge * max(dot(n, rim) + 0.3, 0.0) * 0.28;

    let shin = mix(110.0, 10.0, rough);
    let spec = pow(max(dot(n, normalize(key + v)), 0.0), shin) * (1.0 - rough * 0.8);
    c = c + mix(vec3<f32>(1.0), base, metal) * u.luz_cor.rgb * spec * 0.30 * kd;
    c = c + base * metal * amb * 0.5;
    return to_display(c);
}

// B139: material físico. Microfacetas GGX com sombreamento de Smith (Schlick-GGX) e Fresnel de
// Schlick para a luz principal e o preenchimento; ambiente difuso hemisférico e reflexo do ambiente
// pela aproximação analítica da integral do BRDF (sem textura de consulta). `sh` atenua só a luz
// principal; `ao`, o ambiente.
fn ggx_direta(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, a2: f32, k: f32, f0: vec3<f32>, diffc: vec3<f32>) -> vec3<f32> {
    let nl = max(dot(n, l), 0.0);
    if (nl <= 0.0) { return vec3<f32>(0.0); }
    let h = normalize(l + v);
    let nv = max(dot(n, v), 1e-3);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let dd = nh * nh * (a2 - 1.0) + 1.0;
    let d = a2 / (3.14159265 * dd * dd);
    let g = (nv / (nv * (1.0 - k) + k)) * (nl / (nl * (1.0 - k) + k));
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vh, 5.0);
    let spec = d * g * f / (4.0 * nv * nl + 1e-4);
    return ((vec3<f32>(1.0) - f) * diffc + spec * 3.14159265) * nl;
}
fn brdf_ambiente(f0: vec3<f32>, rough: f32, nv: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = rough * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * nv)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + ab.y;
}
fn shade_pbr(base_s: vec3<f32>, n: vec3<f32>, v: vec3<f32>, rough_in: f32, metal: f32, sh: f32, ao: f32) -> vec3<f32> {
    let base = to_linear(base_s);
    var r = cross(vec3<f32>(0.0, 0.0, 1.0), v);
    if (length(r) < 1e-3) { r = vec3<f32>(1.0, 0.0, 0.0); }
    let right = normalize(r);
    let up = cross(v, right);
    let key = luz_principal(v, right, up);
    let fill = normalize(v * 0.70 + right * 0.70 - up * 0.10);
    let rough = clamp(rough_in, 0.08, 1.0);
    let a2 = pow(rough, 4.0);
    let k = (rough + 1.0) * (rough + 1.0) / 8.0;
    let f0 = mix(vec3<f32>(0.04), base, metal);
    let diffc = base * (1.0 - metal);
    var c = ggx_direta(n, v, key, a2, k, f0, diffc) * vec3<f32>(1.0, 0.98, 0.94) * u.luz_cor.rgb * 0.85 * sh;
    c = c + ggx_direta(n, v, fill, a2, k, f0, diffc) * vec3<f32>(0.80, 0.88, 1.0) * 0.28 * mix(1.0, ao, 0.5);
    let sky = ceu_medio(vec3<f32>(0.62, 0.68, 0.80));
    let ground = select(vec3<f32>(0.20, 0.18, 0.16), sky * 0.45, u.cena.x > 0.5);
    c = c + diffc * mix(ground, sky, n.z * 0.5 + 0.5) * 0.36 * ao * u.cena.w;
    let rf = reflect(-v, n);
    var env = mix(ground, sky, smoothstep(-0.3, 0.5, rf.z)) * mix(1.0, 0.45, rough);
    // B182: com a imagem panorâmica (fora do nível Rápido) o reflexo é a própria imagem, que tende à
    // cor média conforme a rugosidade cresce
    if (u.chao2.w > 0.5) { env = mix(pow(ceu(rf), vec3<f32>(2.2)), sky * 0.8, rough * rough); }
    c = c + brdf_ambiente(f0, rough, max(dot(n, v), 1e-3)) * env * 0.9 * ao;
    return to_display(c);
}

// B425: cor de baixa luminância ganha luz de preenchimento e um contorno claro na silhueta; cor
// parecida com o fundo (peça branca no fundo claro, preta no escuro) ganha contorno do lado oposto
// do fundo. A regra de luminância é a de `contraste.rs`.
fn lum(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }
fn realce_de_contraste(c: vec3<f32>, base: vec3<f32>, n: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    let borda = pow(1.0 - abs(dot(n, v)), 2.5);
    let lb = lum(base);
    let lbg = lum(mix(u.bg_bottom.rgb, u.bg_top.rgb, select(0.0, 0.5, u.bg_bottom.w > 0.5)));
    let escura = 1.0 - smoothstep(0.08, 0.30, lb);
    var o = c + vec3<f32>(0.55, 0.60, 0.72) * escura * (0.06 + 0.22 * borda);
    let perto = 1.0 - smoothstep(0.10, 0.35, abs(lb - lbg));
    let oposto = select(vec3<f32>(0.93, 0.94, 0.97), vec3<f32>(0.06, 0.07, 0.09), lbg > 0.5);
    return mix(o, oposto, perto * borda * 0.55);
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
    if (u.mapa.x > 2.5) { k = min(abs(k1), abs(k2)); } else if (u.mapa.x > 1.5) { k = sqrt(abs(k1 * k2)); } else if (u.mapa.x > 0.5) { k = abs(k1 + k2) * 0.5; }
    return rampa(log2(1.0 + k * u.mapa.y * 15.0) / 4.0);
}

// ---------- fundo
struct BOut { @builtin(position) clip: vec4<f32>, @location(0) y: f32, @location(1) ndc: vec2<f32> };
@vertex fn vs_bg(@builtin(vertex_index) i: u32) -> BOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: BOut;
    o.clip = vec4<f32>(p[i], 0.0, 1.0);
    o.y = p[i].y * 0.5 + 0.5;
    o.ndc = p[i];
    return o;
}
@fragment fn fs_bg(i: BOut) -> @location(0) vec4<f32> {
    // B182: imagem panorâmica pela direção do raio de cada pixel
    if (u.cena.x > 0.5) {
        let a = u.inv_vp * vec4<f32>(i.ndc, 0.0, 1.0);
        let b = u.inv_vp * vec4<f32>(i.ndc, 1.0, 1.0);
        return vec4<f32>(clamp(ceu(normalize(b.xyz / b.w - a.xyz / a.w)), vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
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
    return cor_da_face(i, g, true);
}

// B182: reflexo da peça no chão. A malha vem espelhada pelo plano do chão; longe do centro e fundo
// abaixo dele o reflexo some na cor do chão. A cor sai sem sombra nem oclusão: o ponto espelhado não
// existe na cena, e os mapas de sombra e de profundidade não o conhecem.
@fragment fn fs_espelho(i: MOut) -> @location(0) vec4<f32> {
    let g = cross(dpdx(i.wp), dpdy(i.wp));
    let c = cor_da_face(i, g, false);
    let r = length(i.wp.xy - u.chao.xy) / max(u.chao.w, 1e-3);
    let fundo = (u.chao.z - i.wp.z) / max(u.chao.w * 0.5, 1e-3);
    let some = (1.0 - smoothstep(0.35, 0.8, r)) * (1.0 - smoothstep(0.0, 1.0, fundo));
    return vec4<f32>(mix(cor_do_chao(), c.rgb, some), 1.0);
}

fn cor_da_face(i: MOut, g: vec3<f32>, efeitos: bool) -> vec4<f32> {
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
    // B139: sombra e oclusão só nos sombreados (0, 1) e no semi-renderizado (5)
    var sh = 1.0;
    var ao = 1.0;
    if (efeitos && (mode < 1.5 || (mode > 4.5 && mode < 5.5))) {
        sh = sombra_em(i.wp, n);
        ao = oclusao_em(i.wp, n);
    }
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
        // (B180: a luz e o número de linhas vêm do painel; sem painel, a oblíqua e 14 linhas)
        var l = normalize(vec3<f32>(0.35, -0.45, 0.82));
        if (dot(u.mapa_dir.xyz, u.mapa_dir.xyz) > 0.5 && u.mapa.z > 1.5) { l = normalize(u.mapa_dir.xyz); }
        let s = dot(n, l) * select(14.0, u.mapa.z, u.mapa.z > 1.5);
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
        // zebra: faixas do reflexo de um ambiente de listras ao longo de um eixo do mundo. O número de
        // faixas e o eixo vêm do painel (B180); sem painel, 9 faixas em Z.
        let r = reflect(-v, n);
        var eixo = vec3<f32>(0.0, 0.0, 1.0);
        var nf = 9.0;
        if (u.mapa.z > 1.5) { nf = u.mapa.z; eixo = normalize(u.mapa_dir.xyz); }
        let s = dot(r, eixo) * nf;
        let w = max(fwidth(s), 1e-4);
        let d = abs(dot(n, v));
        if (u.mapa.w > 0.5) {
            // reflexo: tubos de luz suaves sobre um fundo escuro; o brilho segue a curvatura sem degrau
            let t = abs(fract(s) - 0.5) * 2.0;
            let tubo = pow(1.0 - smoothstep(0.0, 0.55, t), 2.0);
            let c = mix(vec3<f32>(0.10, 0.11, 0.13), vec3<f32>(0.96, 0.97, 1.0), tubo);
            return vec4<f32>(c * (0.80 + 0.20 * d), 1.0);
        }
        let band = smoothstep(0.5 - w, 0.5 + w, abs(fract(s) - 0.5) * 2.0);
        return vec4<f32>(vec3<f32>(band * 0.92 + 0.04) * (0.75 + 0.25 * d), 1.0);
    }
    if (mode > 4.5) {
        // semi-renderizado: céu/chão + luz principal + reflexo de ambiente + Fresnel
        let sky = vec3<f32>(0.80, 0.85, 0.95);
        let ground = vec3<f32>(0.22, 0.20, 0.18);
        let amb = mix(ground, sky, n.z * 0.5 + 0.5) * ao * u.cena.w;
        var l = normalize(vec3<f32>(0.45, -0.55, 0.85) + v * 0.6);
        if (u.efeitos.x > 0.5 || u.luz_cor.w > 0.5) { l = normalize(u.luz.xyz); }
        let diff = max(dot(n, l), 0.0) * sh * lum(u.luz_cor.rgb);
        let h = normalize(l + v);
        let shin = mix(400.0, 6.0, rough);
        let spec = pow(max(dot(n, h), 0.0), shin) * (1.0 - rough * 0.85);
        let f0 = mix(vec3<f32>(0.04), base, metal);
        let fres = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
        let r = reflect(-v, n);
        let env = mix(ground, sky, smoothstep(-0.3, 0.4, r.z)) * mix(1.0, ao, 0.7);
        let diffc = base * (1.0 - metal);
        c = diffc * (amb * 0.45 + diff * 0.75) + fres * (env * (1.0 - rough * 0.7) * 0.9 + vec3<f32>(spec) * 1.4 * sh);
        if (metal > 0.5) { c = c + base * env * 0.35; }
    } else if (u.qual.x > 0.5 && u.efeitos.z > 0.5) {
        c = shade_pbr(base, n, v, rough, metal, sh, ao);
    } else if (u.qual.x > 0.5) {
        c = shade_realistic(base, n, v, rough, metal, sh, ao);
    } else {
        let d = abs(dot(n, v));
        let k = normalize(vec3<f32>(0.3, 0.5, 1.0));
        let fill = max(dot(n, k), 0.0) * 0.25;
        let spec = pow(d, 48.0) * 0.22 * (1.0 - rough);
        c = base * (0.30 + 0.55 * d + fill) + vec3<f32>(spec);
    }
    c = realce_de_contraste(c, base, n, v);
    if (mode > 3.5 && mode < 4.5) {
        // raio-X
        c = mix(c, vec3<f32>(0.75, 0.85, 1.0), 0.35);
        alpha = 0.22;
    }
    return vec4<f32>(c, alpha);
}

// ---------- textura de rosca (B351): o vértice leva o eixo vezes o comprimento (n), a origem (mat.xyz) e o passo
// (mat.w, negativo à esquerda). O filete é uma faixa helicoidal por pixel: sem malha de rosca.
fn rosca_textura(i: MOut, v: vec3<f32>, dh: f32, dwp: vec3<f32>) -> vec4<f32> {
    let comp = length(i.n);
    let eixo = i.n / max(comp, 1e-6);
    let passo = abs(i.mat.w);
    let rel = i.wp - i.mat.xyz;
    let h = dot(rel, eixo);
    let rad = rel - eixo * h;
    let r = max(length(rad), 1e-4);
    var a0 = vec3<f32>(1.0, 0.0, 0.0);
    if (abs(eixo.x) > 0.9) { a0 = vec3<f32>(0.0, 1.0, 0.0); }
    let bu = normalize(cross(eixo, a0));
    let bv = cross(eixo, bu);
    let ang = atan2(dot(rad, bv), dot(rad, bu)) / 6.2831853;
    let fase = h / passo - sign(i.mat.w) * ang;
    let d = abs(fract(fase + 0.5) - 0.5);
    // largura do pixel na fase, sem o salto do atan2: só as derivadas de h e do vetor radial
    let aa = max(dh / passo + length(dwp) / (r * 6.2831853), 1e-4);
    let crista = 1.0 - smoothstep(0.17 - aa, 0.17 + aa, d);
    // só dentro do comprimento da rosca
    let fora = smoothstep(0.0, passo * 0.1, h) * (1.0 - smoothstep(comp - passo * 0.1, comp, h));
    let luz = 0.62 + 0.38 * abs(dot(rad / r, v));
    let cor = mix(i.col.rgb * 0.5, i.col.rgb, crista) * luz;
    return vec4<f32>(cor, mix(0.30, i.col.a, crista) * fora);
}

// ---------- fantasma: prévia do comando, na cor que a cena mandou. A borda sai mais densa para o
// contorno aparecer mesmo com a malha quase transparente de frente.
@fragment fn fs_ghost(i: MOut) -> @location(0) vec4<f32> {
    // derivadas antes de qualquer desvio dependente do pixel (exigência de uniformidade do WGSL)
    let dwp = fwidth(i.wp);
    let dh = fwidth(dot(i.wp - i.mat.xyz, normalize(i.n)));
    if (cortado(i.wp)) { discard; }
    let v = normalize(u.eye.xyz - i.wp);
    if (abs(i.mat.w) > 1e-6) { return rosca_textura(i, v, dh, dwp); }
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
// ---------- B139: passadas só de profundidade (mapa de sombra e profundidade para a oclusão)
struct SOut { @builtin(position) clip: vec4<f32> };
@vertex fn vs_sombra(v: MIn) -> SOut {
    var o: SOut;
    o.clip = u.luz_vp * vec4<f32>(v.pos, 1.0);
    return o;
}
@vertex fn vs_sombra_i(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, i: IIn) -> SOut {
    var o: SOut;
    o.clip = u.luz_vp * vec4<f32>(posto(i, vec4<f32>(pos, 1.0)), 1.0);
    return o;
}
// profundidade [0, 1] em 24 bits de uma cor RGBA8 (o alfa fica 1)
fn empacotar(z: f32) -> vec4<f32> {
    let v = floor(clamp(z, 0.0, 1.0) * 16777215.0);
    let a = floor(v / 65536.0);
    let b = floor((v - a * 65536.0) / 256.0);
    return vec4<f32>(a, b, v - a * 65536.0 - b * 256.0, 255.0) / 255.0;
}
fn desempacotar(c: vec4<f32>) -> f32 {
    let v = round(c.xyz * 255.0);
    return (v.x * 65536.0 + v.y * 256.0 + v.z) / 16777215.0;
}
// ---------- B182: chão da cena (dois triângulos gerados no shader) e malha espelhada pelo plano dele
fn espelhar(p: vec3<f32>) -> vec3<f32> { return vec3<f32>(p.x, p.y, 2.0 * u.chao.z - p.z); }
@vertex fn vs_espelho(v: MIn) -> MOut {
    var o: MOut;
    let w = espelhar(v.pos);
    o.clip = u.view_proj * vec4<f32>(w, 1.0);
    o.n = v.nrm * vec3<f32>(1.0, 1.0, -1.0); o.col = v.col; o.wp = w; o.mat = v.mat;
    return o;
}
@vertex fn vs_espelho_i(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, i: IIn) -> MOut {
    var o: MOut;
    let w = espelhar(posto(i, vec4<f32>(pos, 1.0)));
    o.clip = u.view_proj * vec4<f32>(w, 1.0);
    o.n = posto(i, vec4<f32>(nrm, 0.0)) * vec3<f32>(1.0, 1.0, -1.0); o.col = i.col; o.wp = w; o.mat = i.mat;
    return o;
}
struct COut { @builtin(position) clip: vec4<f32>, @location(0) wp: vec3<f32> };
@vertex fn vs_chao(@builtin(vertex_index) k: u32) -> COut {
    var q = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    var o: COut;
    let p = vec3<f32>(u.chao.xy + q[k] * u.chao.w, u.chao.z);
    o.clip = u.view_proj * vec4<f32>(p, 1.0);
    o.wp = p;
    return o;
}
// cor do chão: o fundo puxado para o lado oposto da luminância, para o chão não sumir nele e a peça
// continuar distinta dos dois
fn cor_do_chao() -> vec3<f32> {
    let bgc = mix(u.bg_bottom.rgb, u.bg_top.rgb, select(0.0, 0.5, u.bg_bottom.w > 0.5));
    if (u.cena.x > 0.5) { return u.cena_media.rgb * 0.8; }
    return select(bgc + vec3<f32>(0.07), bgc * 0.84, lum(bgc) > 0.5);
}
@fragment fn fs_chao(i: COut) -> @location(0) vec4<f32> {
    var sh = 1.0;
    if (u.chao2.z > 0.5) { sh = sombra_em(i.wp, vec3<f32>(0.0, 0.0, 1.0)); }
    let cor = cor_do_chao() * mix(0.55, 1.0, sh);
    let r = length(i.wp.xy - u.chao.xy) / max(u.chao.w, 1e-3);
    let borda = 1.0 - smoothstep(0.45, 1.0, r);
    // a sombra fica opaca mesmo com reflexo: senão a peça refletida apagaria a própria sombra
    let a = max(1.0 - u.chao2.y * 0.8, (1.0 - sh) * 0.6);
    return vec4<f32>(cor, a * borda);
}
// a profundidade da câmera respeita o corte da prévia, senão a parte cortada fecharia os cantos
@fragment fn fs_corte(i: MOut) -> @location(0) vec4<f32> {
    if (cortado(i.wp)) { discard; }
    return empacotar(i.clip.z);
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
    /// O modo mede a superfície. O que não é superfície do modelo fica de fora: o shader de análise
    /// devolve cor opaca, então uma sombra translúcida viraria uma chapa pintada por cima da peça.
    pub fn e_analise(self) -> bool {
        matches!(self, DisplayMode::Zebra | DisplayMode::Curvatura | DisplayMode::Inclinacao | DisplayMode::Isofotas | DisplayMode::Dfm)
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
    /// sombra, oclusão e PBR pedidos (B139)
    pub efeitos: crate::render_efeitos::Efeitos,
    /// cena de apresentação do documento (B182): luz principal, ambiente, chão e reflexo
    pub cena: crate::render_efeitos::CenaVista,
}

/// O que o shader precisa para pintar o mapa de curvatura ou de inclinação por pixel.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MapaGpu {
    /// 0 máxima, 1 média, 2 gaussiana, 3 mínima
    pub tipo: f32,
    /// raio que recebe a última cor, em mm
    pub raio: f32,
    /// saída mínima e início da tampa, em graus; na zebra e nas isofotas, faixas e estilo (B180)
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
    luz_vp: [[f32; 4]; 4],
    luz: [f32; 4],
    efeitos: [f32; 4],
    efeitos2: [f32; 4],
    prof: [f32; 4],
    inv_vp: [[f32; 4]; 4],
    cena: [f32; 4],
    cena_media: [f32; 4],
    luz_cor: [f32; 4],
    chao: [f32; 4],
    chao2: [f32; 4],
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
pub(crate) struct Pipes {
    pub(crate) samples: u32,
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
    /// B182: peça espelhada pelo chão (comum e cópias) e o chão, translúcido, sem gravar profundidade
    espelho: wgpu::RenderPipeline,
    espelho_i: wgpu::RenderPipeline,
    chao: wgpu::RenderPipeline,
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
    bgl: wgpu::BindGroupLayout,
    /// B139: recursos da sombra e da oclusão, criados no primeiro quadro que os pede
    efeitos: Option<crate::render_efeitos::Recursos>,
    /// o que a placa aceita; cai para `SemProfundidade` se o módulo dos efeitos falhar
    cap_efeitos: crate::render_efeitos::Capacidade,
    /// aviso de queda dos efeitos, que a interface mostra uma vez
    pub aviso_efeitos: Option<String>,
    aviso_dado: bool,
    /// B182: vistas que o grupo 0 junta (para refazê-lo ao trocar a imagem panorâmica)
    tex_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    /// imagem panorâmica carregada: caminho e cor média (`None` = a de 1 × 1)
    pub panoramica: Option<(String, [u8; 3])>,
}

/// Fonte do shader: o base (sombra e oclusão valem 1) ou o dos efeitos (B139).
pub(crate) fn fonte_shader(efeitos: bool) -> String {
    SHADER.replace(crate::render_efeitos::MARCA, if efeitos { crate::render_efeitos::EFEITOS_WGSL } else { crate::render_efeitos::STUB_WGSL })
}

/// Grupo 0 de recursos: uniforme, texturas e amostrador.
fn grupo0(dev: &wgpu::Device) -> wgpu::BindGroupLayout {
    dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            // B182: imagem panorâmica da cena (1 × 1 enquanto não houver)
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                count: None,
            },
        ],
    })
}

/// Grupo 0 montado: uniforme, texturas das aparências, amostrador e imagem panorâmica.
fn grupo0_bind(dev: &wgpu::Device, bgl: &wgpu::BindGroupLayout, ubuf: &wgpu::Buffer, tex: &wgpu::TextureView, samp: &wgpu::Sampler, pano: &wgpu::TextureView) -> wgpu::BindGroup {
    dev.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tex) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(samp) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(pano) },
        ],
    })
}

/// Lado maior da imagem panorâmica na placa: 2048 × 1024 bastam para fundo e reflexo e cabem no
/// OpenGL ES 3 (2048 é o mínimo garantido).
const PANO_LARGURA: u32 = 2048;

/// Textura da imagem panorâmica (RGBA) e a vista dela.
fn textura_pano(dev: &wgpu::Device, q: &wgpu::Queue, w: u32, h: u32, rgba: &[u8]) -> wgpu::TextureView {
    let t = dev.create_texture(&wgpu::TextureDescriptor {
        label: Some("panoramica"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // sem conversão sRGB: o shader lê valores de tela, como as cores do tema (ver `to_linear`)
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    q.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    t.create_view(&Default::default())
}

/// Lê a imagem panorâmica, reduz para no máximo `PANO_LARGURA` de largura e devolve RGBA, tamanho e
/// a cor média (que vira o fundo para a regra de contraste e a cor do céu na luz de ambiente).
pub fn ler_panoramica(caminho: &std::path::Path) -> Result<(Vec<u8>, [u32; 2], [u8; 3]), String> {
    let img = image::open(caminho).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = img.dimensions();
    let w2 = w.min(PANO_LARGURA).max(2);
    let h2 = ((h as u64 * w2 as u64 / w.max(1) as u64) as u32).clamp(1, PANO_LARGURA / 2);
    let img = if (w2, h2) != (w, h) { image::imageops::resize(&img, w2, h2, image::imageops::FilterType::Triangle) } else { img };
    Ok((img.as_raw().clone(), [w2, h2], media_rgba(img.as_raw())))
}

/// Cor média de uma imagem RGBA.
pub fn media_rgba(rgba: &[u8]) -> [u8; 3] {
    let n = (rgba.len() / 4).max(1) as u64;
    let mut s = [0u64; 3];
    for p in rgba.chunks_exact(4) {
        for k in 0..3 {
            s[k] += p[k] as u64;
        }
    }
    s.map(|x| (x / n) as u8)
}

/// Para o teste: monta o módulo base e o dos efeitos e os pipelines de 1 e 4 amostras num
/// dispositivo qualquer, devolvendo o primeiro erro da placa.
#[cfg(test)]
pub(crate) fn testar_pipelines(dev: &wgpu::Device, q: &wgpu::Queue, cap: crate::render_efeitos::Capacidade) -> Result<(), String> {
    use crate::render_efeitos as ef;
    let bgl = grupo0(dev);
    let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], ..Default::default() });
    let module = ef::protegido(dev, || dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(fonte_shader(false).into()) }))?;
    ef::protegido(dev, || {
        make_pipes(dev, &module, &layout, 1);
        make_pipes(dev, &module, &layout, 4);
    })?;
    let ef::Capacidade::Completa { .. } = cap else { return Ok(()) };
    let mut r = ef::Recursos::criar(dev, &bgl, &module, &layout, &fonte_shader(true))?;
    let a = ef::ativos(ef::Efeitos { nivel: ef::Nivel::Alto, ..Default::default() }, true, false, DisplayMode::SombreadoArestas, false, cap);
    r.garantir(dev, a, [640, 480], 4)?;
    r.garantir(dev, a, [640, 480], 1)?;
    // grava e envia as passadas de profundidade: anexo e pipeline têm de casar
    ef::protegido(dev, || {
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: None, size: std::mem::size_of::<Uniforms>() as u64, usage: wgpu::BufferUsages::UNIFORM, mapped_at_creation: false });
        let tex = dev.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 2 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let tv = tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let samp = dev.create_sampler(&Default::default());
        let pano = textura_pano(dev, q, 2, 1, &[128; 8]);
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tv) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&samp) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&pano) },
            ],
        });
        let mut enc = dev.create_command_encoder(&Default::default());
        r.passadas(&mut enc, a, &bind, None, &crate::instancias::Instanciador::default());
        q.submit([enc.finish()]);
    })
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
pub(crate) fn make_pipes(dev: &wgpu::Device, module: &wgpu::ShaderModule, layout: &wgpu::PipelineLayout, samples: u32) -> Pipes {
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
        espelho: pipe("vs_espelho", "fs_espelho", &mesh_attrs, ms, T::TriangleList, C::Less, true),
        espelho_i: copia("vs_espelho_i", "fs_espelho", T::TriangleList, C::Less, true, true),
        chao: pipe("vs_chao", "fs_chao", &[], 0, T::TriangleList, C::LessEqual, false),
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

    /// B139: decide os efeitos do quadro e garante os recursos deles. Placa sem o recurso, ou erro ao
    /// montar o módulo dos efeitos, derruba sombra e oclusão (o nível cai para Rápido), registra o
    /// motivo e deixa um aviso para a interface, uma vez por sessão.
    fn preparar_efeitos(&mut self, rs: &egui_wgpu::RenderState, style: &ViewStyle, size: [u32; 2], amostras: u32) -> crate::render_efeitos::Ativos {
        use crate::render_efeitos as ef;
        let movendo = style.simplificar && style.navegando;
        let pedidos = ef::ativos(style.efeitos, style.realista, style.leve, style.mode, movendo, ef::Capacidade::Completa { max_px: 4096 });
        let mut a = ef::ativos(style.efeitos, style.realista, style.leve, style.mode, movendo, self.cap_efeitos);
        if pedidos.usa_profundidade() && !a.usa_profundidade() {
            self.avisar_efeitos("placa sem comparação de profundidade ou com textura pequena");
        }
        if !a.usa_profundidade() {
            return a;
        }
        if self.efeitos.is_none() {
            let base_layout = &self.layout;
            match ef::Recursos::criar(&rs.device, &self.bgl, &self.module, base_layout, &fonte_shader(true)) {
                Ok(r) => self.efeitos = Some(r),
                Err(e) => {
                    self.cair_efeitos(&e);
                    a.sombra_px = 0;
                    a.ao_amostras = 0;
                    return a;
                }
            }
        }
        if let Err(e) = self.efeitos.as_mut().unwrap().garantir(&rs.device, a, size, amostras) {
            self.cair_efeitos(&e);
            a.sombra_px = 0;
            a.ao_amostras = 0;
        }
        a
    }

    fn cair_efeitos(&mut self, motivo: &str) {
        self.efeitos = None;
        self.cap_efeitos = crate::render_efeitos::Capacidade::SemProfundidade;
        self.last = None;
        self.avisar_efeitos(motivo);
    }

    fn avisar_efeitos(&mut self, motivo: &str) {
        if self.aviso_dado {
            return;
        }
        self.aviso_dado = true;
        crate::partida::registrar("efeitos", motivo);
        self.aviso_efeitos = Some(tr("A placa de vídeo não aceita sombra e oclusão: a vista passou para o nível Rápido.").to_string());
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
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fenkra"), source: wgpu::ShaderSource::Wgsl(fonte_shader(false).into()) });
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = grupo0(dev);
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
        let pano = textura_pano(dev, &rs.queue, 1, 1, &[128, 128, 128, 255]);
        let bind = grupo0_bind(dev, &bgl, &ubuf, &tex_view, &sampler, &pano);
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
            bgl,
            efeitos: None,
            cap_efeitos: crate::render_efeitos::capacidade(rs.adapter.get_downlevel_capabilities().flags, rs.device.limits().max_texture_dimension_2d),
            aviso_efeitos: None,
            aviso_dado: false,
            tex_view,
            sampler,
            panoramica: None,
        }
    }

    /// B182: troca a imagem panorâmica da cena (`None` volta à de 1 × 1). Devolve o erro de leitura,
    /// e nesse caso a cena fica sem imagem (o fundo volta ao do tema).
    pub fn set_panoramica(&mut self, rs: &egui_wgpu::RenderState, caminho: Option<&str>) -> Result<(), String> {
        let dev = &rs.device;
        let (pano, estado, r) = match caminho.map(|c| ler_panoramica(std::path::Path::new(c))) {
            Some(Ok((rgba, [w, h], media))) => (textura_pano(dev, &rs.queue, w, h, &rgba), Some((caminho.unwrap().to_string(), media)), Ok(())),
            Some(Err(e)) => (textura_pano(dev, &rs.queue, 1, 1, &[128, 128, 128, 255]), None, Err(e)),
            None => (textura_pano(dev, &rs.queue, 1, 1, &[128, 128, 128, 255]), None, Ok(())),
        };
        self.bind = grupo0_bind(dev, &self.bgl, &self.ubuf, &self.tex_view, &self.sampler, &pano);
        self.panoramica = estado;
        self.last = None;
        r
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
        // B139: o que vale neste quadro; sem recurso na placa o nível cai sozinho e avisa uma vez
        let ativos = self.preparar_efeitos(rs, &style, size, amostras);
        let ue = crate::render_efeitos::uniforme(ativos, &self.camera, self.bounds, style.cena.luz_fixa.map(Vec3::from));
        let t = self.targets.as_ref().unwrap();
        let vp = self.camera.view_proj(size[0] as f32 / size[1] as f32);
        let e = self.camera.eye();
        // B182: o que da cena vale neste quadro (chão, reflexo, sombra no chão), pelo nível e pelo modo
        let mut cv = style.cena;
        if self.panoramica.is_none() {
            cv.panoramica = None;
        }
        let abaixo = self.bounds.is_some_and(|b| e.z < crate::render_efeitos::plano_do_chao(b)[2]);
        let cq = crate::render_efeitos::cena_do_quadro(&cv, style.efeitos, ativos, style.leve, style.mode, style.simplificar && style.navegando, abaixo);
        let uc = crate::render_efeitos::uniforme_cena(&cv, cq, vp, self.bounds, self.panoramica.as_ref().map_or([128; 3], |p| p.1));
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
            luz_vp: ue.luz_vp,
            luz: ue.luz,
            efeitos: ue.efeitos,
            efeitos2: ue.efeitos2,
            prof: ue.prof,
            inv_vp: uc.inv_vp,
            cena: uc.cena,
            cena_media: uc.cena_media,
            luz_cor: uc.luz_cor,
            chao: uc.chao,
            chao2: uc.chao2,
        };
        rs.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));

        let (Some(ms), Some(p)) = (t.ms.iter().find(|m| m.samples == amostras), self.pipes.iter().find(|p| p.samples == amostras)) else { return t.tex_id };
        let mut enc = rs.device.create_command_encoder(&Default::default());
        // B139: mapa de sombra e profundidade da câmera antes da passada principal
        let efeito = if ativos.usa_profundidade() { self.efeitos.as_ref().and_then(|r| Some((r, r.pipes(amostras)?, r.bind()?))) } else { None };
        if let Some((r, _, _)) = efeito {
            let malha = self.vb.as_ref().filter(|_| style.mode != DisplayMode::Arame).map(|(v, i, n)| (v, i, *n));
            crate::medir_quadros::fase("efeitos", || r.passadas(&mut enc, ativos, &self.bind, malha, &self.inst));
        }
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
            // B139: as faces opacas usam o módulo dos efeitos (grupo 1) quando sombra ou oclusão valem
            let (mesh, mesh_i, chao) = match efeito {
                Some((_, pe, b1)) if m != DisplayMode::RaioX => {
                    pass.set_bind_group(1, b1, &[]);
                    (&pe.mesh, &pe.mesh_i, &pe.chao)
                }
                _ => (&p.mesh, &p.mesh_i, &p.chao),
            };
            // B182: a peça espelhada vem antes, embaixo do chão; o chão translúcido passa por cima dela
            if faces && cq.chao && cq.reflexo > 0. {
                if let Some((vb, ib, n)) = &self.vb {
                    pass.set_pipeline(&p.espelho);
                    pass.set_vertex_buffer(0, vb.slice(..));
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..*n, 0, 0..1);
                }
                pass.set_pipeline(&p.espelho_i);
                self.inst.faces(&mut pass);
            }
            if let (true, Some((vb, ib, n))) = (faces, &self.vb) {
                pass.set_pipeline(if m == DisplayMode::RaioX { &p.xray } else { mesh });
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..*n, 0, 0..1);
            }
            if faces {
                pass.set_pipeline(if m == DisplayMode::RaioX { &p.xray_i } else { mesh_i });
                self.inst.faces(&mut pass);
            }
            // o chão depois das faces opacas: a peça o esconde onde está na frente dele
            if uc.chao2[0] > 0.5 {
                pass.set_pipeline(chao);
                pass.draw(0..6, 0..1);
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
