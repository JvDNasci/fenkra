// SPDX-License-Identifier: LGPL-2.1-or-later
//! Prévia do filete e do chanfro no 3D (B289): realce da cadeia tangente inteira e, quando o valor
//! não cabe, a superfície que o comando tentou fazer.
//!
//! Como funciona: com a propagação tangente ligada, o kernel arredonda também as arestas que
//! continuam a escolhida sem quebra de direção; por isso a prévia realça a cadeia toda, não só a
//! aresta clicada. Quando o cálculo falha, o kernel não devolve geometria; a prévia refaz a conta
//! simples da seção: o filete é um arco de raio R tangente às duas faces (o tangenciamento fica a
//! R·tan(φ/2) da aresta, com φ o ângulo entre as normais) e o chanfro, um segmento entre os pontos a
//! D de cada lado. Desenhadas em algumas seções ao longo da aresta, as linhas mostram onde a
//! superfície sai da face vizinha, que é o motivo da falha.

use glam::Vec3;

type Poli = Vec<Vec3>;

fn v(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// Arestas da cadeia tangente da escolhida, sem a própria escolhida.
pub fn cadeia_tangente(m: &fenkra_kernel::Mesh, aresta: usize) -> Vec<usize> {
    crate::picking::tangent_edges_mesh(m, aresta).1.into_iter().filter(|e| *e != aresta).collect()
}

/// Direção, sobre a face `fi`, que se afasta da aresta no ponto `p` (rumo ao centro da face), dada a
/// direção `t` da aresta e a normal `n` da face.
pub(crate) fn afasta_da_aresta(m: &fenkra_kernel::Mesh, fi: usize, p: Vec3, t: Vec3, n: Vec3) -> Option<Vec3> {
    let centro = centro_da_face(m, fi)?;
    t.cross(n).try_normalize().map(|u| if u.dot(centro - p) < 0. { -u } else { u })
}

/// Centro da face `fi` (média dos vértices dos triângulos dela).
fn centro_da_face(m: &fenkra_kernel::Mesh, fi: usize) -> Option<Vec3> {
    let i0 = *m.face_ranges.get(fi)? as usize;
    let i1 = m.face_ranges.get(fi + 1).map(|x| *x as usize).unwrap_or(m.indices.len());
    let (mut soma, mut n) = (Vec3::ZERO, 0.);
    for i in &m.indices[i0..i1] {
        let k = *i as usize * 3;
        soma += Vec3::new(m.positions[k], m.positions[k + 1], m.positions[k + 2]);
        n += 1.;
    }
    (n > 0.).then(|| soma / n)
}

/// `n` pontos a distâncias iguais ao longo da polilinha (os dois extremos incluídos).
fn amostrar(poly: &[Vec3], n: usize) -> Vec<Vec3> {
    let total: f32 = poly.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    if total < 1e-6 || n < 2 {
        return poly.to_vec();
    }
    (0..n)
        .map(|k| {
            let alvo = total * k as f32 / (n - 1) as f32;
            let mut acc = 0.;
            for w in poly.windows(2) {
                let l = (w[1] - w[0]).length();
                if acc + l >= alvo - 1e-6 || std::ptr::eq(w, &poly[poly.len() - 2..]) {
                    return w[0] + (w[1] - w[0]) * if l > 1e-9 { ((alvo - acc) / l).clamp(0., 1.) } else { 0. };
                }
                acc += l;
            }
            *poly.last().unwrap()
        })
        .collect()
}

/// Linhas da superfície que o filete (`chanfro` = falso, `valor` = raio) ou o chanfro (`valor` =
/// distância) tentou fazer na aresta `e`, em até 6 seções mais as linhas que as ligam. Vazio quando
/// a aresta não tem duas faces ou elas são tangentes.
pub fn superficie_tentada(m: &fenkra_kernel::Mesh, e: usize, valor: f64, chanfro: bool) -> Vec<Poli> {
    let Some([fa, fb]) = m.edge_faces.get(e).copied().filter(|p| p[0] != fenkra_kernel::NO_FACE && p[1] != fenkra_kernel::NO_FACE) else { return vec![] };
    let (fa, fb) = (fa as usize, fb as usize);
    let bruta: Vec<Vec3> = crate::picking::edge_polyline_mesh(m, e).into_iter().map(v).collect();
    if bruta.len() < 2 || !(valor > 0.) {
        return vec![];
    }
    // seis seções a distâncias iguais ao longo da aresta (uma reta só tem os dois pontos das pontas)
    let poly = amostrar(&bruta, 6);
    let (Some(ca), Some(cb)) = (centro_da_face(m, fa), centro_da_face(m, fb)) else { return vec![] };
    let n = poly.len();
    let estacoes: Vec<usize> = (0..n).collect();
    let r = valor as f32;
    let mut secoes: Vec<Poli> = vec![];
    // pontos de cada seção que as linhas longitudinais ligam: contato em A, meio e contato em B
    let mut trilhos: [Poli; 3] = Default::default();
    for &i in &estacoes {
        let p = poly[i];
        let t = (poly[(i + 1).min(n - 1)] - poly[i.saturating_sub(1)]).try_normalize();
        let (Some(t), Some(na), Some(nb)) = (t, super::handles::solidos::normal_do_indice(m, fa, [p.x as f64, p.y as f64, p.z as f64]), super::handles::solidos::normal_do_indice(m, fb, [p.x as f64, p.y as f64, p.z as f64])) else { continue };
        let cos_phi = na.dot(nb).clamp(-1., 1.);
        if cos_phi > 0.9995 {
            continue;
        }
        let phi = cos_phi.acos();
        // direção, sobre cada face, que se afasta da aresta (rumo ao centro da face)
        let afasta = |n: Vec3, centro: Vec3| t.cross(n).try_normalize().map(|u| if u.dot(centro - p) < 0. { -u } else { u });
        let (Some(ua), Some(ub)) = (afasta(na, ca), afasta(nb, cb)) else { continue };
        // aresta convexa: sair pela face A leva ao lado de dentro da B; côncava, ao de fora
        let convexa = ua.dot(nb) < 0.;
        let tang = r * (phi / 2.).tan();
        let (pa, pb) = (p + ua * tang, p + ub * tang);
        let (sec, meio) = if chanfro {
            let (qa, qb) = (p + ua * r, p + ub * r);
            (vec![qa, qb], (qa + qb) / 2.)
        } else {
            let sinal = if convexa { -1. } else { 1. };
            let centro = pa + na * (sinal * r);
            let (va, vb) = (na * (-sinal * r), nb * (-sinal * r));
            // arco de A até B girando em torno da aresta (interpolação esférica das duas normais)
            let sin_phi = phi.sin().max(1e-6);
            let arco: Poli = (0..=16)
                .map(|k| {
                    let s = k as f32 / 16.;
                    centro + (va * ((1. - s) * phi).sin() + vb * (s * phi).sin()) / sin_phi
                })
                .collect();
            let meio = arco[8];
            (arco, meio)
        };
        let (cont_a, cont_b) = (sec[0], *sec.last().unwrap());
        trilhos[0].push(cont_a);
        trilhos[1].push(meio);
        trilhos[2].push(cont_b);
        secoes.push(sec);
    }
    secoes.extend(trilhos.into_iter().filter(|t| t.len() > 1));
    secoes
}

/// Põe as polilinhas na lista de linhas (pares de pontos) da cena.
pub fn empurrar(dst: &mut Vec<crate::render::LineVertex>, polis: &[Poli], col: [f32; 4]) {
    for p in polis {
        for w in p.windows(2) {
            dst.push(crate::render::LineVertex { pos: w[0].into(), col });
            dst.push(crate::render::LineVertex { pos: w[1].into(), col });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fenkra_core::*;
    use fenkra_solidos::recompute;

    /// Caixa 40 × 30 × 20: o único corpo, de onde sai a malha de antes de qualquer filete.
    fn caixa() -> Document {
        let mut d = Document::new("caixa");
        let mut data = SketchData::default();
        let mut al = || d.alloc();
        data.add_rect(&mut al, [-20., -15.], [20., 15.], false);
        let sk = d.add_feature(FeatureKind::Sketch(Sketch { plane: PlaneRef::Origin(OriginPlane::XY), data }));
        d.add_feature(FeatureKind::Extrude(Extrude { sketch: sk, op: BoolOp::Add, extent: Extent::Blind("20".into()), reverse: false, taper: "0".into(), thin: None, dir: None, thin_side: 0, ends: Default::default() }));
        d
    }

    #[test]
    fn superficie_tentada_do_filete_e_um_arco_tangente_as_duas_faces() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&caixa());
        let m = &r.bodies[0].mesh;
        // aresta do topo da frente: y = -15, z = 20, de x = -20 a 20
        let (e, _) = crate::entradas::aresta_perto(m, [0., -15., 20.]).expect("aresta");
        let polis = superficie_tentada(m, e, 6., false);
        // seis seções de arco e três linhas que as ligam
        assert_eq!(polis.len(), 6 + 3, "{}", polis.len());
        let arco = &polis[0];
        assert_eq!(arco.len(), 17);
        // o arco começa e termina nas faces: 6 mm da aresta, sobre z = 20 e sobre y = -15
        let (a, b) = (arco[0], *arco.last().unwrap());
        let (topo, frente) = if (a.z - 20.).abs() < 1e-3 { (a, b) } else { (b, a) };
        assert!((topo.z - 20.).abs() < 1e-3 && (topo.y + 15. - 6.).abs() < 1e-3, "{topo:?}");
        assert!((frente.y + 15.).abs() < 1e-3 && (frente.z - 20. + 6.).abs() < 1e-3, "{frente:?}");
        // todo ponto do arco está a 6 mm do centro (-15 + 6, 20 - 6)
        let centro = Vec3::new(topo.x, -9., 14.);
        assert!(arco.iter().all(|p| ((*p - Vec3::new(p.x, centro.y, centro.z)).length() - 6.).abs() < 1e-3), "{arco:?}");
        // as seções ficam entre x = -20 e 20, ao longo da aresta
        assert!(polis[..6].iter().any(|s| (s[0].x + 20.).abs() < 1e-3) && polis[..6].iter().any(|s| (s[0].x - 20.).abs() < 1e-3));
    }

    #[test]
    fn superficie_tentada_do_chanfro_liga_os_pontos_a_distancia_d_de_cada_lado() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&caixa());
        let m = &r.bodies[0].mesh;
        let (e, _) = crate::entradas::aresta_perto(m, [0., -15., 20.]).expect("aresta");
        let polis = superficie_tentada(m, e, 4., true);
        let seg = &polis[0];
        assert_eq!(seg.len(), 2);
        let (topo, frente) = if (seg[0].z - 20.).abs() < 1e-3 { (seg[0], seg[1]) } else { (seg[1], seg[0]) };
        assert!((topo.y + 15. - 4.).abs() < 1e-3 && (frente.z - 16.).abs() < 1e-3, "{seg:?}");
    }

    #[test]
    fn aresta_sem_duas_faces_ou_valor_invalido_nao_desenha() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        let r = recompute(&caixa());
        let m = &r.bodies[0].mesh;
        let (e, _) = crate::entradas::aresta_perto(m, [0., -15., 20.]).expect("aresta");
        assert!(superficie_tentada(m, e, 0., false).is_empty());
        assert!(superficie_tentada(m, 9999, 3., false).is_empty());
    }

    #[test]
    fn cadeia_tangente_inclui_as_arestas_que_continuam_a_escolhida() {
        if !fenkra_kernel::AVAILABLE {
            return;
        }
        // caixa com o canto vertical arredondado: a aresta do topo, a do arco e a da outra face
        // formam uma cadeia só
        let mut d = caixa();
        d.add_feature(FeatureKind::Fillet(EdgeOp { value: "5".into(), edges: EdgeSel::Edges(vec![[20., -15., 10.]]), mode: EdgeMode::Constant, prop: Default::default(), keys: vec![], extra: Default::default() }));
        let r = recompute(&d);
        let m = &r.bodies[0].mesh;
        let (e, _) = crate::entradas::aresta_perto(m, [0., -15., 20.]).expect("aresta do topo");
        let cadeia = cadeia_tangente(m, e);
        assert!(cadeia.len() >= 2, "a reta de cima continua no arco do canto e na reta lateral: {cadeia:?}");
        assert!(!cadeia.contains(&e));
    }
}
