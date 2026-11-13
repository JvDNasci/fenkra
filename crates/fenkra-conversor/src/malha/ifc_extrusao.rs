// SPDX-License-Identifier: LGPL-2.1-or-later
//! Sólido por extrusão do IFC (`IfcExtrudedAreaSolid`) virando malha (B211).
//!
//! É a forma mais comum nos arquivos de edificação: um perfil plano empurrado ao longo de uma
//! direção. O perfil vem de `IfcRectangleProfileDef`, `IfcCircleProfileDef` (polígono de 64 lados) ou
//! `IfcArbitraryClosedProfileDef` com polilinha ou curva indexada só de retas. O contorno é
//! triangulado por orelhas (aceita perfil côncavo, como um L ou um U), as tampas fecham e as laterais
//! são quadriláteros. Perfil com furos (`...WithVoids`) entra só pelo contorno de fora, e o
//! relatório conta o furo que ficou de fora.

/// Lados do polígono que representa um círculo de perfil.
pub(super) const LADOS_CIRCULO: usize = 64;

/// Área com sinal de um polígono no plano (positiva no sentido anti-horário).
pub(super) fn area(p: &[[f64; 2]]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>() / 2.
}

fn cruz(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn dentro(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    cruz(a, b, p) >= -1e-12 && cruz(b, c, p) >= -1e-12 && cruz(c, a, p) >= -1e-12
}

/// Triangulação por orelhas de um polígono simples em sentido anti-horário. Devolve índices.
pub(super) fn orelhas(p: &[[f64; 2]]) -> Vec<[u32; 3]> {
    let mut resto: Vec<usize> = (0..p.len()).collect();
    let mut out = vec![];
    let mut guarda = 0;
    while resto.len() > 3 && guarda < p.len() * p.len() + 8 {
        guarda += 1;
        let n = resto.len();
        let mut cortou = false;
        for k in 0..n {
            let (ia, ib, ic) = (resto[(k + n - 1) % n], resto[k], resto[(k + 1) % n]);
            let (a, b, c) = (p[ia], p[ib], p[ic]);
            if cruz(a, b, c) <= 1e-12 {
                continue;
            }
            if resto.iter().any(|&j| j != ia && j != ib && j != ic && dentro(p[j], a, b, c)) {
                continue;
            }
            out.push([ia as u32, ib as u32, ic as u32]);
            resto.remove(k);
            cortou = true;
            break;
        }
        if !cortou {
            break;
        }
    }
    if resto.len() == 3 {
        out.push([resto[0] as u32, resto[1] as u32, resto[2] as u32]);
    } else if resto.len() > 3 {
        // polígono que se cruza: o leque fecha a malha sem travar
        for k in 1..resto.len() - 1 {
            out.push([resto[0] as u32, resto[k] as u32, resto[k + 1] as u32]);
        }
    }
    out
}

/// Malha do perfil `contorno` (no plano XY do sólido) empurrado por `dir` × `prof`, já na posição
/// `lugar` (matriz 3x4 por linhas).
pub(super) fn extrudar(contorno: &[[f64; 2]], dir: [f64; 3], prof: f64, lugar: &[f64; 12]) -> Option<super::Malha> {
    let mut c: Vec<[f64; 2]> = contorno.to_vec();
    // o último ponto repete o primeiro nas polilinhas fechadas
    if c.len() > 1 && (c[0][0] - c[c.len() - 1][0]).abs() < 1e-12 && (c[0][1] - c[c.len() - 1][1]).abs() < 1e-12 {
        c.pop();
    }
    if c.len() < 3 || !(prof.is_finite() && prof > 0.) {
        return None;
    }
    if area(&c) < 0. {
        c.reverse();
    }
    if area(&c).abs() < 1e-12 {
        return None;
    }
    let n = c.len();
    let d = {
        let k = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if k < 1e-12 {
            return None;
        }
        dir.map(|x| x / k * prof)
    };
    // a direção pode apontar para baixo do plano: a tampa de cima troca de lado
    let para_cima = d[2] >= 0.;
    let aplicar = |p: [f64; 3]| fenkra_malha::cena::aplicar(lugar, p);
    let mut pos = Vec::with_capacity(2 * n);
    for p in &c {
        pos.push(aplicar([p[0], p[1], 0.]));
    }
    for p in &c {
        pos.push(aplicar([p[0] + d[0], p[1] + d[1], d[2]]));
    }
    let base = orelhas(&c);
    let mut tris = vec![];
    for t in &base {
        let (fundo, topo) = ([t[0], t[2], t[1]], [t[0] + n as u32, t[1] + n as u32, t[2] + n as u32]);
        if para_cima {
            tris.push(fundo);
            tris.push(topo);
        } else {
            tris.push([t[0], t[1], t[2]]);
            tris.push([topo[0], topo[2], topo[1]]);
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b, a2, b2) = (i as u32, j as u32, (i + n) as u32, (j + n) as u32);
        if para_cima {
            tris.push([a, b, b2]);
            tris.push([a, b2, a2]);
        } else {
            tris.push([a, b2, b]);
            tris.push([a, a2, b2]);
        }
    }
    // escala negativa (espelho) no lugar inverte o sentido das faces
    if fenkra_malha::cena::determinante(lugar) < 0. {
        for t in &mut tris {
            t.swap(1, 2);
        }
    }
    Some(super::Malha { pos, tris, ..Default::default() })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn perfil_em_l_concavo_fecha_com_o_volume_certo() {
        let l = [[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 2.], [0., 2.]];
        let t = orelhas(&l);
        assert_eq!(t.len(), 4);
        let m = extrudar(&l, [0., 0., 1.], 5., &fenkra_malha::IDENTIDADE).unwrap().soldada();
        assert!(m.fechada());
        assert!((m.volume() - 4. * 5.).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn sentido_horario_e_direcao_para_baixo_continuam_com_volume_positivo() {
        let q = [[0., 0.], [0., 2.], [2., 2.], [2., 0.]];
        let m = extrudar(&q, [0., 0., -1.], 3., &fenkra_malha::IDENTIDADE).unwrap().soldada();
        assert!(m.fechada());
        assert!((m.volume() - 12.).abs() < 1e-9, "{}", m.volume());
        assert!(extrudar(&q, [0., 0., 1.], 0., &fenkra_malha::IDENTIDADE).is_none());
    }
}
