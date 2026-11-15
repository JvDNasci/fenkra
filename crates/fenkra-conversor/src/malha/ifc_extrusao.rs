// SPDX-License-Identifier: LGPL-2.1-or-later
//! Sólido por extrusão do IFC (`IfcExtrudedAreaSolid`) virando malha (B211).
//!
//! É a forma mais comum nos arquivos de edificação: um perfil plano empurrado ao longo de uma
//! direção. O perfil vem de `IfcRectangleProfileDef`, `IfcCircleProfileDef` (polígono de 64 lados), dos
//! perfis de aço paramétricos (I, L, U, T, tubo retangular e redondo) ou de `IfcArbitraryClosedProfileDef`
//! com polilinha ou curva indexada (o arco pelos três pontos vira polígono com o mesmo passo do círculo).
//!
//! O contorno é triangulado por orelhas (aceita perfil côncavo, como um L ou um U). Perfil com furos
//! (`...WithVoids` e os tubos) liga cada furo ao contorno de fora por uma ponte, do vértice mais à direita
//! do furo ao vértice visível mais próximo, e o polígono único que sai daí também vai por orelhas. As
//! tampas fecham e as laterais são quadriláteros, por fora e por dentro de cada furo. Furo sem ponte
//! visível (um furo que cruza o contorno) fica de fora e volta contado para o relatório.

/// Lados do polígono que representa um círculo de perfil.
pub(super) const LADOS_CIRCULO: usize = 64;

type P2 = [f64; 2];

/// Área com sinal de um polígono no plano (positiva no sentido anti-horário).
pub(super) fn area(p: &[P2]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>() / 2.
}

fn cruz(o: P2, a: P2, b: P2) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn dentro(p: P2, a: P2, b: P2, c: P2) -> bool {
    cruz(a, b, p) >= -1e-12 && cruz(b, c, p) >= -1e-12 && cruz(c, a, p) >= -1e-12
}

fn igual(a: P2, b: P2) -> bool {
    (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
}

/// Triangulação por orelhas de um polígono simples em sentido anti-horário. Devolve índices. Vértices
/// repetidos (as duas pontas de cada ponte de furo) não barram a orelha um do outro.
pub(super) fn orelhas(p: &[P2]) -> Vec<[u32; 3]> {
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
            if resto.iter().any(|&j| j != ia && j != ib && j != ic && !igual(p[j], a) && !igual(p[j], b) && !igual(p[j], c) && dentro(p[j], a, b, c)) {
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

/// Pontos do arco que passa por `p1`, `p2` e `p3`, de `p1` a `p3`, com o passo do círculo de perfil.
/// Três pontos em linha voltam como estão.
pub(super) fn arco_por_tres(p1: P2, p2: P2, p3: P2) -> Vec<P2> {
    let d = 2. * (p1[0] * (p2[1] - p3[1]) + p2[0] * (p3[1] - p1[1]) + p3[0] * (p1[1] - p2[1]));
    if d.abs() < 1e-12 {
        return vec![p1, p2, p3];
    }
    let q = |p: P2| p[0] * p[0] + p[1] * p[1];
    let c = [
        (q(p1) * (p2[1] - p3[1]) + q(p2) * (p3[1] - p1[1]) + q(p3) * (p1[1] - p2[1])) / d,
        (q(p1) * (p3[0] - p2[0]) + q(p2) * (p1[0] - p3[0]) + q(p3) * (p2[0] - p1[0])) / d,
    ];
    let r = (p1[0] - c[0]).hypot(p1[1] - c[1]);
    let ang = |p: P2| (p[1] - c[1]).atan2(p[0] - c[0]);
    let tau = std::f64::consts::TAU;
    let (a1, a3) = (ang(p1), ang(p3));
    // o sentido é o que passa pelo ponto do meio
    let anti_horario = cruz(p1, p2, p3) > 0.;
    let mut varre = a3 - a1;
    if anti_horario {
        while varre <= 0. {
            varre += tau;
        }
    } else {
        while varre >= 0. {
            varre -= tau;
        }
    }
    let n = ((varre.abs() / tau * LADOS_CIRCULO as f64).ceil() as usize).max(2);
    let mut pts: Vec<P2> = (0..n).map(|i| a1 + varre * i as f64 / n as f64).map(|t| [c[0] + r * t.cos(), c[1] + r * t.sin()]).collect();
    pts.push(p3);
    pts
}

/// Laço limpo: sem o último ponto repetindo o primeiro, com área de verdade.
fn limpo(c: &[P2]) -> Option<Vec<P2>> {
    let mut c = c.to_vec();
    if c.len() > 1 && igual(c[0], c[c.len() - 1]) {
        c.pop();
    }
    (c.len() >= 3 && area(&c).abs() >= 1e-12).then_some(c)
}

/// `a`-`b` cruza `c`-`d` por dentro (tocar na ponta não conta).
fn cruzam(a: P2, b: P2, c: P2, d: P2) -> bool {
    let (d1, d2, d3, d4) = (cruz(c, d, a), cruz(c, d, b), cruz(a, b, c), cruz(a, b, d));
    d1 * d2 < -1e-18 && d3 * d4 < -1e-18
}

/// `h` cai no setor de dentro do polígono anti-horário no vértice `v` (entre `antes` e `depois`).
fn no_setor(antes: P2, v: P2, depois: P2, h: P2) -> bool {
    if cruz(antes, v, depois) >= 0. {
        cruz(antes, v, h) > 0. && cruz(v, depois, h) > 0.
    } else {
        cruz(antes, v, h) > 0. || cruz(v, depois, h) > 0.
    }
}

/// Liga os furos ao contorno de fora e devolve o polígono único, em índices dos pontos de `pts`, e quantos
/// furos ficaram de fora. `lacos` dá o começo e o tamanho de cada laço em `pts`; o primeiro é o de fora.
fn com_pontes(pts: &[P2], lacos: &[(usize, usize)]) -> (Vec<usize>, usize) {
    let (i0, n0) = lacos[0];
    let mut poli: Vec<usize> = (i0..i0 + n0).collect();
    let mut furos: Vec<(usize, usize)> = lacos[1..].to_vec();
    let mais_a_direita = |&(ini, n): &(usize, usize)| (ini..ini + n).max_by(|a, b| pts[*a][0].total_cmp(&pts[*b][0])).unwrap_or(ini);
    furos.sort_by(|a, b| pts[mais_a_direita(b)][0].total_cmp(&pts[mais_a_direita(a)][0]));
    let mut fora = 0;
    for (k, furo) in furos.iter().enumerate() {
        let h = mais_a_direita(furo);
        let ph = pts[h];
        let m = poli.len();
        let mut candidatos: Vec<usize> = (0..m).collect();
        candidatos.sort_by(|a, b| {
            let d = |i: &usize| (pts[poli[*i]][0] - ph[0]).hypot(pts[poli[*i]][1] - ph[1]);
            d(a).total_cmp(&d(b))
        });
        // arestas que a ponte não pode cruzar: o polígono de agora e os furos que faltam
        let mut arestas: Vec<(P2, P2)> = (0..m).map(|i| (pts[poli[i]], pts[poli[(i + 1) % m]])).collect();
        for &(ini, n) in &furos[k..] {
            arestas.extend((0..n).map(|i| (pts[ini + i], pts[ini + (i + 1) % n])));
        }
        let ponte = candidatos.into_iter().find(|&i| {
            let v = pts[poli[i]];
            no_setor(pts[poli[(i + m - 1) % m]], v, pts[poli[(i + 1) % m]], ph) && !arestas.iter().any(|&(c, d)| cruzam(v, ph, c, d))
        });
        let Some(i) = ponte else {
            fora += 1;
            continue;
        };
        let (ini, n) = *furo;
        let volta: Vec<usize> = (0..=n).map(|j| ini + (h - ini + j) % n).collect();
        let mut novo = poli[..=i].to_vec();
        novo.extend(volta);
        novo.push(poli[i]);
        novo.extend_from_slice(&poli[i + 1..]);
        poli = novo;
    }
    (poli, fora)
}

/// Malha dos `contornos` do perfil (o de fora primeiro e os furos depois, no plano XY do sólido)
/// empurrados por `dir` × `prof`, já na posição `lugar` (matriz 3x4 por linhas). Devolve também quantos
/// furos ficaram de fora.
pub(super) fn extrudar(contornos: &[Vec<P2>], dir: [f64; 3], prof: f64, lugar: &[f64; 12]) -> Option<(super::Malha, usize)> {
    if !(prof.is_finite() && prof > 0.) {
        return None;
    }
    let mut fora = limpo(contornos.first()?)?;
    if area(&fora) < 0. {
        fora.reverse();
    }
    let mut lacos_pts = vec![fora];
    let mut furos_fora = 0;
    for f in &contornos[1..] {
        match limpo(f) {
            Some(mut f) => {
                // furo no sentido horário: o mesmo cálculo das laterais vira a parede de dentro
                if area(&f) > 0. {
                    f.reverse();
                }
                lacos_pts.push(f);
            }
            None => furos_fora += 1,
        }
    }
    let d = {
        let k = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if k < 1e-12 {
            return None;
        }
        dir.map(|x| x / k * prof)
    };
    let mut pts: Vec<P2> = Vec::new();
    let mut lacos = Vec::new();
    for l in &lacos_pts {
        lacos.push((pts.len(), l.len()));
        pts.extend_from_slice(l);
    }
    let (poli, sem_ponte) = com_pontes(&pts, &lacos);
    furos_fora += sem_ponte;
    let n = pts.len() as u32;
    // a direção pode apontar para baixo do plano: a tampa de cima troca de lado
    let para_cima = d[2] >= 0.;
    let aplicar = |p: [f64; 3]| fenkra_malha::cena::aplicar(lugar, p);
    let mut pos = Vec::with_capacity(2 * pts.len());
    for p in &pts {
        pos.push(aplicar([p[0], p[1], 0.]));
    }
    for p in &pts {
        pos.push(aplicar([p[0] + d[0], p[1] + d[1], d[2]]));
    }
    let coords: Vec<P2> = poli.iter().map(|i| pts[*i]).collect();
    let mut tris = vec![];
    for t in orelhas(&coords) {
        let t = t.map(|i| poli[i as usize] as u32);
        let (fundo, topo) = ([t[0], t[2], t[1]], [t[0] + n, t[1] + n, t[2] + n]);
        if para_cima {
            tris.push(fundo);
            tris.push(topo);
        } else {
            tris.push(t);
            tris.push([topo[0], topo[2], topo[1]]);
        }
    }
    // só as laterais dos laços que entraram no polígono (o furo sem ponte não tem tampa)
    let usados: std::collections::HashSet<usize> = poli.iter().copied().collect();
    for &(ini, m) in lacos.iter().filter(|(ini, _)| usados.contains(ini)) {
        for i in 0..m {
            let j = (i + 1) % m;
            let (a, b) = ((ini + i) as u32, (ini + j) as u32);
            let (a2, b2) = (a + n, b + n);
            if para_cima {
                tris.push([a, b, b2]);
                tris.push([a, b2, a2]);
            } else {
                tris.push([a, b2, b]);
                tris.push([a, a2, b2]);
            }
        }
    }
    // escala negativa (espelho) no lugar inverte o sentido das faces
    if fenkra_malha::cena::determinante(lugar) < 0. {
        for t in &mut tris {
            t.swap(1, 2);
        }
    }
    Some((super::Malha { pos, tris, ..Default::default() }, furos_fora))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn extr(c: &[Vec<P2>], dir: [f64; 3], prof: f64) -> (super::super::Malha, usize) {
        let (m, f) = extrudar(c, dir, prof, &fenkra_malha::IDENTIDADE).unwrap();
        (m.soldada(), f)
    }

    #[test]
    fn perfil_em_l_concavo_fecha_com_o_volume_certo() {
        let l = vec![[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 2.], [0., 2.]];
        assert_eq!(orelhas(&l).len(), 4);
        let (m, _) = extr(&[l], [0., 0., 1.], 5.);
        assert!(m.fechada());
        assert!((m.volume() - 4. * 5.).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn sentido_horario_e_direcao_para_baixo_continuam_com_volume_positivo() {
        let q = vec![[0., 0.], [0., 2.], [2., 2.], [2., 0.]];
        let (m, _) = extr(&[q.clone()], [0., 0., -1.], 3.);
        assert!(m.fechada());
        assert!((m.volume() - 12.).abs() < 1e-9, "{}", m.volume());
        assert!(extrudar(&[q], [0., 0., 1.], 0., &fenkra_malha::IDENTIDADE).is_none());
    }

    #[test]
    fn placa_com_dois_furos_fecha_e_desconta_a_area_dos_furos() {
        let fora = vec![[0., 0.], [10., 0.], [10., 6.], [0., 6.]];
        let quadrado = |x: f64, y: f64| vec![[x, y], [x + 2., y], [x + 2., y + 2.], [x, y + 2.]];
        // um furo no sentido anti-horário e outro no horário: o sentido do arquivo não importa
        let mut f2 = quadrado(6., 2.);
        f2.reverse();
        let (m, fora_n) = extr(&[fora, quadrado(1., 2.), f2], [0., 0., 1.], 5.);
        assert_eq!(fora_n, 0);
        assert!(m.fechada());
        assert!((m.volume() - (60. - 8.) * 5.).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn furo_que_cruza_o_contorno_fica_de_fora_e_e_contado() {
        let fora = vec![[0., 0.], [10., 0.], [10., 6.], [0., 6.]];
        let furo = vec![[8., 2.], [12., 2.], [12., 4.], [8., 4.]];
        let (m, fora_n) = extr(&[fora, furo], [0., 0., 1.], 1.);
        assert_eq!(fora_n, 1);
        assert!((m.volume() - 60.).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn arco_por_tres_pontos_fica_no_circulo_e_segue_o_ponto_do_meio() {
        // meia volta de cima, de (1,0) por (0,1) até (-1,0)
        let p = arco_por_tres([1., 0.], [0., 1.], [-1., 0.]);
        assert!(p.iter().all(|q| (q[0].hypot(q[1]) - 1.).abs() < 1e-12));
        assert!(p.iter().all(|q| q[1] >= -1e-12), "o arco passa por cima");
        assert_eq!(p.len(), LADOS_CIRCULO / 2 + 1);
        // o mesmo arco por baixo
        let q = arco_por_tres([1., 0.], [0., -1.], [-1., 0.]);
        assert!(q.iter().all(|p| p[1] <= 1e-12));
    }
}
