//! Chapa metálica: apoio puro (sem kernel) para as abas.
//!
//! - `chain_segs`: transforma a polilinha de uma cadeia de arestas (retas e curvas tangentes) em
//!   segmentos exatos (reta, arco ou spline), na mesma regra de `sm_chain_sweep` do kernel.
//! - `colisao` e `valida_lateral`: validações das abas (folga real entre abas e lateral = espessura).

use crate::geo3d;
use fenkra_kernel::{Bool, Seg2, Shape};

type V3 = [f64; 3];

/// Volume comum (mm³) acima do qual a aba nova conta como colisão com a chapa ou com outra aba.
const VOLUME_COLISAO: f64 = 1e-3;

/// Compensação de uma dobra: a única conta de comprimento desenvolvido do módulo. A geometria
/// (abas, dobra por linha, joggle), as linhas de dobra, a tabela de dobras e o DXF leem daqui, para
/// que nenhum deles discorde dos outros.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Compensacao {
    /// comprimento da dobra na linha neutra (bend allowance)
    pub ba: f64,
    /// recuo externo (distância do vértice das faces de fora ao começo da dobra)
    pub ossb: f64,
    /// dedução de dobra: 2 × recuo externo − ba
    pub deducao: f64,
}

/// `ang` em graus (o sinal não importa), `r` raio interno, `k` fator K já resolvido para esse raio
/// (veja `SheetValues::k_for`). Perto de 180° o recuo externo tende ao infinito e é limitado a r + t.
pub fn compensacao(t: f64, r: f64, ang: f64, k: f64) -> Compensacao {
    let ang = ang.abs();
    let a = ang.to_radians();
    let ba = a * (r + k * t);
    let ossb = if ang < 179.9 { (a / 2.).tan() * (r + t) } else { r + t };
    Compensacao { ba, ossb, deducao: 2. * ossb - ba }
}

/// Linhas de dobra de um sketch (cadeias abertas): cada reta é uma linha. Curva suave (arco ou
/// spline) vira a corda dela se o desvio for pequeno (até 1 % da corda, no mínimo 0,1 mm), com
/// aviso; mais que isso exigiria uma dobra cônica, que o comando ainda não faz, e vira erro.
pub fn linhas_dobra(open: &[Vec<[f64; 2]>]) -> Result<(Vec<([f64; 2], [f64; 2])>, Vec<String>), String> {
    let mut linhas = vec![];
    let mut avisos = vec![];
    let desvio = |pts: &[[f64; 2]]| -> f64 {
        let (a, b) = (pts[0], *pts.last().unwrap());
        let c = (b[0] - a[0]).hypot(b[1] - a[1]).max(1e-12);
        pts.iter().map(|p| ((p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])).abs() / c).fold(0., f64::max)
    };
    for cadeia in open.iter().filter(|c| c.len() >= 2) {
        for seg in chain_segs(cadeia) {
            let (pts, curva): (Vec<[f64; 2]>, bool) = match seg {
                // uma curva suave amostrada cabe toda no limite de "reta" de `chain_segs` (desvio por
                // vértice pequeno): os pontos originais entre as pontas decidem se ainda é reta
                Seg2::Line(a, b) => {
                    let i0 = cadeia.iter().position(|q| *q == a);
                    let i1 = cadeia.iter().rposition(|q| *q == b);
                    match (i0, i1) {
                        (Some(i), Some(j)) if j > i + 1 => (cadeia[i..=j].to_vec(), desvio(&cadeia[i..=j]) > 1e-3),
                        _ => (vec![a, b], false),
                    }
                }
                Seg2::Arc(a, m, b) => (vec![a, m, b], true),
                Seg2::Spline(p) => (p, true),
                Seg2::Circle(..) => continue,
            };
            let (a, b) = (pts[0], *pts.last().unwrap());
            let corda = (b[0] - a[0]).hypot(b[1] - a[1]);
            if corda < 1e-6 {
                continue;
            }
            if curva {
                let d = desvio(&pts);
                if d > (corda * 0.01).max(0.1) {
                    return Err(format!("a linha de dobra curva desvia {d:.2} mm da corda de {corda:.1} mm: uma dobra em curva forte pede peça cônica, ainda sem suporte. Use retas ou uma curva quase reta"));
                }
                avisos.push(format!("linha curva aproximada pela corda (desvio de {d:.2} mm)"));
            }
            linhas.push((a, b));
        }
    }
    if linhas.is_empty() {
        return Err("o sketch da dobra precisa de linhas abertas (retas ou curva quase reta)".into());
    }
    Ok((linhas, avisos))
}

/// A lateral da aresta escolhida (a face de ponta) tem a espessura da chapa? `None` = sim.
pub fn valida_lateral(medida: f64, t: f64) -> Option<String> {
    if medida <= 0. || (medida - t).abs() <= 0.02 * t.max(1.) {
        return None;
    }
    Some(format!("a aresta escolhida tem lateral de {medida:.2} mm, diferente da espessura da chapa ({t:.2} mm): escolha a aresta de ponta da chapa"))
}

/// Colisão da aba nova (`tool`) com o corpo atual: a aba só pode encostar na chapa pela aresta, então
/// qualquer volume comum é atrito entre abas. Devolve a mensagem com a folga real que falta (recuo
/// lateral necessário na ponta onde bate). `ends` são as pontas da aba (já com os recuos atuais).
pub fn colisao(body: &Shape, tool: &Shape, ends: [V3; 2], off1: f64, off2: f64) -> Result<Option<String>, String> {
    let hit = tool.boolean(body, Bool::Common).map_err(|e| e.0)?;
    if hit.is_null() {
        return Ok(None);
    }
    let vol = hit.mass().volume;
    if vol <= VOLUME_COLISAO {
        return Ok(None);
    }
    let Some(e) = geo3d::unit(geo3d::sub(ends[1], ends[0])) else {
        return Ok(Some(format!("a aba colide com a chapa ou com outra aba (volume comum de {vol:.2} mm³)")));
    };
    let len = geo3d::dist(ends[0], ends[1]);
    let mut dica = String::new();
    if let Some((lo, hi)) = hit.bbox() {
        // extensão da sobreposição ao longo da aresta, medida a partir da ponta 0
        let (mut s0, mut s1) = (f64::MAX, f64::MIN);
        for i in 0..8 {
            let c = [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }];
            let s = geo3d::dot(geo3d::sub(c, ends[0]), e);
            s0 = s0.min(s);
            s1 = s1.max(s);
        }
        if s0 < len * 0.5 && s1 < len * 0.5 + 1e-9 {
            dica = format!("; recue a ponta 1 para {:.2} mm", off1 + s1.max(0.))
        } else if s0 > len * 0.5 - 1e-9 {
            dica = format!("; recue a ponta 2 para {:.2} mm", off2 + (len - s0).max(0.))
        }
    }
    Ok(Some(format!("a aba colide com a chapa ou com outra aba (volume comum de {vol:.2} mm³){dica}")))
}


/// Desvio de direção (rad) abaixo do qual um vértice conta como reta (~0,5°).
const RETA: f64 = 0.0087;

fn ang(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let u = [b[0] - a[0], b[1] - a[1]];
    let v = [c[0] - b[0], c[1] - b[1]];
    let n = (u[0].hypot(u[1])) * (v[0].hypot(v[1]));
    if n < 1e-18 {
        return 0.;
    }
    ((u[0] * v[0] + u[1] * v[1]) / n).clamp(-1., 1.).acos()
}

/// Círculo por três pontos: centro e raio (`None` se colineares).
fn circulo(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<([f64; 2], f64)> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    Some(([ux, uy], (a[0] - ux).hypot(a[1] - uy)))
}

/// Cadeia plana (retas e curvas tangentes) em segmentos exatos: vértices com desvio menor que ~0,5°
/// são reta; um trecho de vértices com desvio maior vira arco (se os pontos caem num círculo) ou
/// spline. Uma spline única por todos os pontos oscila na emenda reta-curva.
pub fn chain_segs(v: &[[f64; 2]]) -> Vec<Seg2> {
    let mut p: Vec<[f64; 2]> = vec![];
    for q in v {
        if p.last().is_none_or(|l| (l[0] - q[0]).hypot(l[1] - q[1]) > 1e-9) {
            p.push(*q);
        }
    }
    let n = p.len();
    if n < 2 {
        return vec![];
    }
    let curved: Vec<bool> = (0..n).map(|i| i > 0 && i + 1 < n && ang(p[i - 1], p[i], p[i + 1]) > RETA).collect();
    let mut out = vec![];
    let mut i = 0;
    while i + 1 < n {
        let mut c0 = i + 1;
        while c0 < n - 1 && !curved[c0] {
            c0 += 1;
        }
        if c0 >= n - 1 {
            out.push(Seg2::Line(p[i], p[n - 1]));
            break;
        }
        let mut c1 = c0;
        while c1 + 1 < n - 1 && curved[c1 + 1] {
            c1 += 1;
        }
        if p[i] != p[c0] {
            out.push(Seg2::Line(p[i], p[c0]));
        }
        if c1 == c0 {
            i = c0;
            continue;
        }
        let m = (c0 + c1) / 2;
        let fits = circulo(p[c0], p[m], p[c1]).is_some_and(|(c, r)| (c0..=c1).all(|j| ((p[j][0] - c[0]).hypot(p[j][1] - c[1]) - r).abs() < (r * 2e-3).max(1e-3)));
        if fits {
            out.push(Seg2::Arc(p[c0], p[m], p[c1]));
        } else {
            out.push(Seg2::Spline(p[c0..=c1].to_vec()));
        }
        i = c1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reta_arco_reta_vira_tres_segmentos() {
        let mut v = vec![[0., 0.], [10., 0.]];
        for i in 1..=12 {
            let t = i as f64 / 12. * std::f64::consts::FRAC_PI_2;
            v.push([10. + 5. * t.sin(), 5. - 5. * t.cos()]);
        }
        v.push([15., 20.]);
        let s = chain_segs(&v);
        assert_eq!(s.len(), 3, "{s:?}");
        assert!(matches!(s[0], Seg2::Line(..)) && matches!(s[1], Seg2::Arc(..)) && matches!(s[2], Seg2::Line(..)));
    }

    #[test]
    fn reta_pura_e_spline() {
        assert_eq!(chain_segs(&[[0., 0.], [5., 0.], [10., 0.]]).len(), 1);
        // curva que não é círculo: spline
        let v: Vec<[f64; 2]> = (0..=20).map(|i| [i as f64, (i as f64 * 0.4).sin() * 3. + (i as f64 * 0.15).powi(2)]).collect();
        let s = chain_segs(&v);
        assert!(s.iter().any(|x| matches!(x, Seg2::Spline(_))), "{s:?}");
    }
}
