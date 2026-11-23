// SPDX-License-Identifier: LGPL-2.1-or-later
//! B556: limites e cantos da aba por superfície.
//!
//! Como funciona no kernel:
//!
//! - **Lados 1 e 2.** A linha de dobra (OML) é a interseção do suporte com o plano da web. Cada
//!   limite de lado vira uma curva nesse mesmo plano: um plano ou uma superfície é cortado pelo
//!   plano da web (`section`); uma curva, aresta, linha ou sketch é projetada nele, normal à
//!   web. As duas pontas são estendidas pelas tangentes (o suporte e o limite sempre tentam se
//!   encontrar), a interseção mais perto de cada ponta da OML é escolhida e a OML é aparada
//!   entre as duas. A aba nasce só nesse trecho. Sem interseção, o comando avisa.
//! - **Borda final.** O limite vira uma superfície de corte (plano ou superfície como estão; curva
//!   estendida normal à aba). A parede é varrida mais comprida que o preciso (medindo, com raios,
//!   quanto falta até o limite) e a ferramenta da aba é dividida por ele antes de se unir à
//!   chapa: fica o lado da raiz. Assim a aba se estende ou se recorta até o limite.
//! - **Cantos.** Filete ou chanfro nas duas arestas curtas (do tamanho da espessura) em que a face
//!   da ponta encontra a face do lado. A aresta é achada por um raio que sobe pelo meio da parede.

use super::*;

/// Recortes e cantos que a aba por superfície pede à varredura da aba curva.
#[derive(Default)]
pub(crate) struct AbaLim {
    /// superfície de corte da borda final e um ponto da raiz (o lado que fica)
    pub ponta: Option<(Shape, V3)>,
    /// superfícies de corte dos lados 1 e 2 (a varredura passa do limite e é cortada nele, para o
    /// fim da aba seguir o limite mesmo oblíquo) e um ponto do lado que fica
    pub lados: Vec<(Shape, V3)>,
    /// canto da ponta 1 e da ponta 2: (chanfro?, valor em mm)
    pub cantos: [Option<(bool, f64)>; 2],
}

/// Onde achar as arestas dos cantos de cada ponta da aba.
pub(crate) struct CantoGeo {
    /// por ponta: ponto interno da dobra, direção para fora da chapa e direção para dentro da aba
    pub pontas: [(V3, V3, V3); 2],
    /// sentido da aba (normal da chapa do lado em que ela sobe)
    pub d: V3,
    pub sv: f64,
    /// meio da parede e sentido dela, no plano da seção
    pub meio: [f64; 2],
    pub sobe: [f64; 2],
    pub espessura: f64,
}

fn proj2(q: V3, on: V3, x: V3, y: V3) -> [f64; 2] {
    let d = geo3d::sub(q, on);
    [geo3d::dot(d, x), geo3d::dot(d, y)]
}

/// Cruzamento de dois segmentos no plano: parâmetro em ab e em cd.
fn cruza2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<(f64, f64)> {
    let (r, s) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let ac = [c[0] - a[0], c[1] - a[1]];
    let u = (ac[0] * s[1] - ac[1] * s[0]) / den;
    let v = (ac[0] * r[1] - ac[1] * r[0]) / den;
    ((-1e-9..=1. + 1e-9).contains(&u) && (-1e-9..=1. + 1e-9).contains(&v)).then_some((u, v))
}

/// Estende as pontas de uma polilinha pelas tangentes (em `n` partes), exceto se fechada.
fn estende(p: &[V3], por: f64, n: usize) -> Vec<V3> {
    if p.len() < 2 || geo3d::dist(p[0], p[p.len() - 1]) < 1e-9 {
        return p.to_vec();
    }
    let m = (p.len() / 10).max(1).min(p.len() - 1);
    let (Some(ts), Some(te)) = (geo3d::unit(geo3d::sub(p[0], p[m])), geo3d::unit(geo3d::sub(p[p.len() - 1], p[p.len() - 1 - m]))) else { return p.to_vec() };
    let mut out: Vec<V3> = (1..=n).rev().map(|i| geo3d::add(p[0], geo3d::mul(ts, por * i as f64 / n as f64))).collect();
    out.extend_from_slice(p);
    out.extend((1..=n).map(|i| geo3d::add(p[p.len() - 1], geo3d::mul(te, por * i as f64 / n as f64))));
    out
}

/// Põe a OML num sentido que não depende da ordem do kernel: a ponta de menor coordenada (X, depois
/// Y, depois Z) é a ponta 1.
pub(crate) fn orienta_oml(p: &mut [V3]) {
    if p.len() < 2 {
        return;
    }
    let d = geo3d::sub(p[p.len() - 1], p[0]);
    if let Some(c) = d.iter().find(|c| c.abs() > 1e-6) {
        if *c < 0. {
            p.reverse();
        }
    }
}

/// Apara a OML (já orientada) entre os limites das pontas 1 e 2, cada um dado como curvas no plano
/// da web. Estende a OML e o limite pelas tangentes até se encontrarem.
pub(crate) fn aparar_oml(oml: &[V3], on: V3, n: V3, l1: Option<&[Vec<V3>]>, l2: Option<&[Vec<V3>]>, ext: f64, margem: f64) -> Result<(Vec<V3>, V3), String> {
    let x = geo3d::any_perp(n);
    let y = fenkra_kernel::cross(n, x);
    let pe = estende(oml, ext, 8);
    let antes = (pe.len() - oml.len()) / 2;
    // comprimento acumulado da OML estendida
    let mut s = vec![0.; pe.len()];
    for i in 1..pe.len() {
        s[i] = s[i - 1] + geo3d::dist(pe[i - 1], pe[i]);
    }
    let (s_ini, s_fim) = (s[antes], s[antes + oml.len() - 1]);
    let p2: Vec<[f64; 2]> = pe.iter().map(|q| proj2(*q, on, x, y)).collect();
    let acha = |lim: &[Vec<V3>], perto: f64, nome: &str| -> Result<f64, String> {
        let mut best: Option<f64> = None;
        for c in lim {
            let c = estende(c, ext * 2., 1);
            let c2: Vec<[f64; 2]> = c.iter().map(|q| proj2(*q, on, x, y)).collect();
            for i in 0..p2.len() - 1 {
                for j in 0..c2.len().saturating_sub(1) {
                    if let Some((u, _)) = cruza2d(p2[i], p2[i + 1], c2[j], c2[j + 1]) {
                        let sc = s[i] + u * (s[i + 1] - s[i]);
                        if best.is_none_or(|b| (sc - perto).abs() < (b - perto).abs()) {
                            best = Some(sc);
                        }
                    }
                }
            }
        }
        best.ok_or_else(|| format!("o limite do {nome} não intercepta o suporte, nem estendido: escolha outro elemento ou outro plano"))
    };
    let a = match l1 {
        Some(l) => acha(l, s_ini, "lado 1")?,
        None => s_ini,
    };
    let b = match l2 {
        Some(l) => acha(l, s_fim, "lado 2")?,
        None => s_fim,
    };
    if b - a < 1e-3 {
        return Err("os limites dos lados 1 e 2 se cruzam ou coincidem: não sobra aba entre eles".into());
    }
    let em = |t: f64| -> V3 {
        let i = s.partition_point(|v| *v <= t).clamp(1, s.len() - 1);
        let f = ((t - s[i - 1]) / (s[i] - s[i - 1]).max(1e-12)).clamp(0., 1.);
        geo3d::lerp(pe[i - 1], pe[i], f)
    };
    // a ponta limitada passa do limite por `margem` (a aba é cortada nele depois)
    let (a2, b2) = (if l1.is_some() { (a - margem).max(0.) } else { a }, if l2.is_some() { (b + margem).min(s[s.len() - 1]) } else { b });
    let mut out = vec![em(a2)];
    out.extend((0..pe.len()).filter(|i| s[*i] > a2 + 1e-6 && s[*i] < b2 - 1e-6).map(|i| pe[i]));
    out.push(em(b2));
    Ok((out, em((a + b) / 2.)))
}

impl Ctx<'_> {
    /// Limite de lado como curvas no plano da web (`on`, `n`): superfície cortada pelo plano; curva
    /// projetada nele, normal à web.
    pub(crate) fn aba_limite_no_plano(&self, lim: &LimiteAba, on: V3, n: V3, big: f64) -> Result<Vec<Vec<V3>>, String> {
        let k = |e: KernelError| e.0;
        match lim {
            LimiteAba::Superficie(s) => {
                let sh = self.cut_shape(s, 0).map_err(|e| format!("o limite não está disponível ({e}): escolha de novo"))?;
                let plano = Shape::plane_face(on, n, big).map_err(k)?;
                let v = sh.section(&plano).map_err(k)?;
                if v.is_empty() {
                    return Err("o limite não corta o plano da web".into());
                }
                Ok(v)
            }
            LimiteAba::Curva(c) => {
                let cv = self.curve(c, 0).map_err(|e| format!("o limite não está disponível ({e}): escolha de novo"))?;
                let p: Vec<V3> = cv.pts.iter().map(|q| geo3d::sub(*q, geo3d::mul(n, geo3d::dot(geo3d::sub(*q, on), n)))).collect();
                Ok(vec![p])
            }
        }
    }

    /// Superfície de corte da borda final. A curva é estendida normal à parede (`nw`) para virar
    /// superfície.
    pub(crate) fn aba_cortador_ponta(&self, lim: &LimiteAba, nw: V3, big: f64) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        Ok(match lim {
            LimiteAba::Superficie(s) => self.cut_shape(s, 0).map_err(|e| format!("o limite da borda final não está disponível ({e}): escolha de novo"))?,
            LimiteAba::Curva(c) => {
                let cv = self.curve(c, 0).map_err(|e| format!("o limite da borda final não está disponível ({e}): escolha de novo"))?;
                let p: Vec<V3> = cv.pts.iter().map(|q| geo3d::sub(*q, geo3d::mul(nw, big))).collect();
                Shape::extrude_curve(&p, nw, big * 2.).map_err(k)?
            }
        })
    }

    /// Corta a ferramenta (a varredura da aba ou o recorte do recuo) nos limites dos lados: fica o lado da
    /// raiz. Corte que não toca a ferramenta (a OML acabou antes) deixa a ponta como está.
    pub(crate) fn aba_corta_lados(&self, t: Shape, lim: &AbaLim) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let mut t = t;
        for (cut, raiz) in &lim.lados {
            let (p0, nl) = cut.surface_near(*raiz).map_err(k)?;
            let lado = geo3d::dot(nl, geo3d::sub(*raiz, p0));
            if lado.abs() < 1e-6 {
                continue;
            }
            if let Ok(c) = t.split_side(cut, lado > 0., true) {
                if !c.is_null() && c.face_count() > 0 && c.mass().volume > 1e-3 {
                    t = c;
                }
            }
        }
        Ok(t)
    }

    /// Recorta a ferramenta da aba pelo limite da borda final e faz os cantos das pontas.
    pub(crate) fn aba_pos_ferramenta(&self, t: Shape, lim: &AbaLim, g: &CantoGeo) -> Result<Shape, String> {
        let k = |e: KernelError| e.0;
        let mut t = self.aba_corta_lados(t, lim)?;
        if let Some((cut, raiz)) = &lim.ponta {
            let (p0, nl) = cut.surface_near(*raiz).map_err(k)?;
            let lado = geo3d::dot(nl, geo3d::sub(*raiz, p0));
            if lado.abs() < 1e-6 {
                return Err("a raiz da aba está sobre o limite da borda final: escolha outro limite".into());
            }
            let v0 = t.mass().volume;
            let s = t.split_side(cut, lado > 0., true).map_err(|e| format!("o limite da borda final não corta a aba ({})", e.0))?;
            let v1 = if s.is_null() || s.face_count() == 0 { 0. } else { s.mass().volume };
            if v1 >= v0 - 1e-4 * v0.abs().max(1.) {
                return Err("o limite da borda final não intercepta a aba (nem estendida)".into());
            }
            if v1 < 0.02 * v0 {
                return Err("o limite da borda final deixa a aba sem material: ele fica antes da dobra".into());
            }
            t = s;
        }
        for (i, c) in lim.cantos.iter().enumerate() {
            let Some((chanfro, v)) = c else { continue };
            let (ini, fora, dentro) = g.pontas[i];
            let p3 = |q: [f64; 2]| geo3d::add(geo3d::add(ini, geo3d::mul(fora, q[0])), geo3d::mul(g.d, g.sv * q[1]));
            let dir = geo3d::add(geo3d::mul(fora, g.sobe[0]), geo3d::mul(g.d, g.sv * g.sobe[1]));
            // 1) do meio da parede, logo acima da dobra, para dentro da aba: a face da ponta
            let rente = geo3d::add(p3(g.meio), geo3d::mul(dir, 1.));
            let ini_raio = geo3d::sub(rente, geo3d::mul(dentro, 0.3));
            let h1 = t.ray_hits(ini_raio, dentro).map_err(k)?;
            let Some(h1) = h1.into_iter().find(|h| *h > 1e-6) else {
                return Err(format!("não achei a ponta {} da aba para o canto", i + 1));
            };
            let na_face = geo3d::add(ini_raio, geo3d::mul(dentro, h1));
            // 2) dali, subindo pela parede um pouco para dentro: a face da borda final
            let base = geo3d::add(na_face, geo3d::mul(dentro, 0.05));
            let hits = t.ray_hits(base, dir).map_err(k)?;
            let alt = hits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if !alt.is_finite() || alt <= 0. {
                return Err(format!("não achei o canto da ponta {} para o {}", i + 1, if *chanfro { "chanfro" } else { "filete" }));
            }
            let mid = geo3d::add(na_face, geo3d::mul(dir, alt));
            let nome = if *chanfro { "chanfro" } else { "filete" };
            let (s, _) = t.edges_tagged(*v, &[mid], g.espessura * 0.5 + 0.2, *chanfro, &[]).map_err(|e| format!("não foi possível fazer o {nome} no canto do lado {} ({}): o valor {v:.2} não cabe: tente um valor menor", i + 1, e.0))?;
            t = s;
        }
        Ok(t)
    }

    /// Cantos da faixa planificada (a ponta livre de cada extremo).
    pub(crate) fn aba_cantos_planos(&self, t: Shape, lim: &AbaLim, pontas: [(V3, V3); 2], dev: f64, d: V3, esp: f64) -> Result<Shape, String> {
        let mut t = t;
        for (i, c) in lim.cantos.iter().enumerate() {
            let Some((chanfro, v)) = c else { continue };
            let (ini, fora) = pontas[i];
            let mid = geo3d::sub(geo3d::add(ini, geo3d::mul(fora, dev)), geo3d::mul(d, esp / 2.));
            let nome = if *chanfro { "chanfro" } else { "filete" };
            let (s, _) = t.edges_tagged(*v, &[mid], esp * 0.5 + 0.2, *chanfro, &[]).map_err(|e| format!("não foi possível fazer o {nome} no canto do lado {} ({}): o valor {v:.2} não cabe: tente um valor menor", i + 1, e.0))?;
            t = s;
        }
        Ok(t)
    }
}
