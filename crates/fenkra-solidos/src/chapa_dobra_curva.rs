// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobras que não giram a chapa por um eixo reto: cadeia curva, joggle e dobra por linha em curva.
//!
//! Elas entram no mapa de dobras como linha (B77), sem mapa de giro: uma chapa só gira em torno de
//! reta. Por isso a dobra por linha em curva (B03) não gira a parte de fora: corta a chapa na curva
//! e levanta uma parede varrida ao longo dela, com a mesma conta de compensação das outras dobras.

use super::*;

/// Passos de bisseção para achar onde a curva sai da chapa: leva 1 m a menos de 0,001 mm.
const BISSECOES: usize = 30;

/// Tolerância para dizer que um ponto da face está sobre a chapa.
const SOBRE: f64 = 1e-3;

impl Ctx<'_> {
    /// Registra uma dobra só como linha. `linhas` traz o começo, o centro e o fim da zona dobrada, no
    /// estado atual da chapa; o mapa guarda tudo no espaço planificado.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn marca_dobra(&mut self, feature: Id, linhas: [&[V3]; 3], ang: f64, r: f64, up: bool, cmp: &chapa::Compensacao, flat: bool) {
        let plano = |c: &Self, l: &[V3]| -> Vec<V3> { l.iter().map(|q| c.map_point(*q, flat, true)).collect() };
        let [inicio, centro, fim] = linhas.map(|l| plano(self, l));
        let (Some(a), Some(b)) = (centro.first().copied(), centro.last().copied()) else { return };
        let corda = |l: &[V3]| [l[0], l[l.len() - 1]];
        let bl = BendLine {
            start: corda(&inicio),
            center: [a, b],
            end: corda(&fim),
            angle: ang.abs(),
            radius: r,
            up,
            feature,
            allowance: cmp.ba,
            deduction: cmp.deducao,
            eixo: None,
            curva: if centro.len() > 2 { centro.clone() } else { vec![] },
        };
        self.cut_outlines.push(CutOutline { feature, folded: vec![], flat: vec![], dobra: Some(bl) });
    }

    /// Dobra ao longo de uma cadeia: `base` é onde a zona começa e `saida` aponta para onde ela anda.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn marca_cadeia(&mut self, feature: Id, base: &[V3], saida: &[V3], ang: f64, r: f64, up: bool, cmp: &chapa::Compensacao, flat: bool) {
        let at = |d: f64| -> Vec<V3> { base.iter().zip(saida).map(|(p, o)| geo3d::add(*p, geo3d::mul(*o, d))).collect() };
        let (a, b, c) = (at(0.), at(cmp.ba / 2.), at(cmp.ba));
        self.marca_dobra(feature, [&a, &b, &c], ang, r, up, cmp, flat);
    }

    /// Cadeia tangente viva que passa por `ponto`, na ordem da cadeia guardada `cache`. A polilinha
    /// gravada é só cache: o contorno pode ter mudado de forma. `None` quando a cadeia se ramifica.
    pub(crate) fn cadeia_tangente(&self, target: Id, ponto: V3, cache: &[V3]) -> Option<Vec<V3>> {
        let (b, _, _) = self.body_owned(target)?;
        let topo = b.topology().ok()?;
        let cs: Vec<Curve3> = topo.edge_mids.iter().filter_map(|m| b.edge_at(*m).ok()).filter_map(|p| Curve3::poly(p, false).ok()).collect();
        let perto = |c: &Curve3| geo3d::dist(c.nearest(ponto).1, ponto);
        let start = (0..cs.len()).min_by(|x, y| perto(&cs[*x]).total_cmp(&perto(&cs[*y])))?;
        if perto(&cs[start]) > SOBRE {
            return None;
        }
        let indices = geo3d::tangent_chain(&cs, start, SOBRE);
        let mut cadeia = encadeia(indices.iter().map(|i| cs[*i].pts.clone()).collect())?;
        // mesmo sentido do cache: a aba sai para o mesmo lado
        if let (Some(a), Some(z)) = (cache.first(), cache.last()) {
            let direto = geo3d::dist(cadeia[0], *a) + geo3d::dist(cadeia[cadeia.len() - 1], *z);
            let contrario = geo3d::dist(cadeia[0], *z) + geo3d::dist(cadeia[cadeia.len() - 1], *a);
            if contrario < direto {
                cadeia.reverse();
            }
        }
        Some(cadeia)
    }

    /// Dobra por linha em curva. `pts` é a linha já no estado atual; ela marca o centro da zona, ou
    /// o que `position` disser, igual à dobra reta.
    pub(crate) fn dobra_curva(&mut self, f: &Feature, pts: Vec<V3>, ang: f64, r: f64, bd: &SmBend) -> Result<Option<String>, String> {
        let k = |e: KernelError| e.0;
        let sp = self.sheet()?;
        let b = self.body(f.target).ok_or("a dobra precisa de uma chapa")?.clone();
        let (on, n) = b.surface_near(pts[pts.len() / 2]).map_err(k)?;
        let plano: Vec<V3> = pts.iter().map(|p| geo3d::sub(*p, geo3d::mul(n, geo3d::dot(geo3d::sub(*p, on), n)))).collect();
        let tan = |i: usize| geo3d::unit(geo3d::sub(plano[(i + 1).min(plano.len() - 1)], plano[i.saturating_sub(1)]));
        let sinal = if bd.flip { -1. } else { 1. };
        let saida: Vec<V3> = (0..plano.len()).map(|i| tan(i).and_then(|t| geo3d::unit(fenkra_kernel::cross(n, t))).map(|s| geo3d::mul(s, sinal))).collect::<Option<_>>().ok_or("a linha de dobra tem pontos repetidos")?;
        let cmp = chapa::compensacao(sp.t, r, ang, sp.k_for(r));
        let recuo = chapa_linha::deslocamento_posicao(bd.position, &cmp) - cmp.ba / 2.;
        let zona: Vec<V3> = plano.iter().zip(&saida).map(|(p, o)| geo3d::add(*p, geo3d::mul(*o, recuo))).collect();
        let (lo, hi) = b.bbox().ok_or("corpo sem dimensões")?;
        let big = geo3d::dist(lo, hi) * 2. + 10.;
        let ultimo = zona.len() - 1;
        let antes = geo3d::add(zona[0], geo3d::mul(tan(0).ok_or("linha de dobra nula")?, -big));
        let depois = geo3d::add(zona[ultimo], geo3d::mul(tan(ultimo).ok_or("linha de dobra nula")?, big));
        let linha: Vec<V3> = [vec![antes], zona.clone(), vec![depois]].concat();
        let cadeia = trecho_na_chapa(&linha, &|q| b.surface_near(q).is_ok_and(|(s, _)| geo3d::dist(s, q) < SOBRE)).ok_or("a linha de dobra em curva não passa pela chapa")?;
        let saida_cadeia: Vec<V3> = cadeia.iter().map(|q| saida[nearest_index(&zona, *q)]).collect();
        if sp.flat {
            self.marca_cadeia(f.id, &cadeia, &saida_cadeia, ang, r, !bd.down, &cmp, true);
            return Ok(Some("planificada: dobra desfeita".into()));
        }
        // região além da curva: separa a parte que vira parede
        let fr = Frame { origin: on, x: geo3d::any_perp(n), normal: n };
        let y = fenkra_kernel::cross(fr.normal, fr.x);
        let loc = |p: V3| {
            let q = geo3d::sub(p, fr.origin);
            [geo3d::dot(q, fr.x), geo3d::dot(q, y)]
        };
        let p2: Vec<[f64; 2]> = linha.iter().map(|p| loc(*p)).collect();
        let (fim, comeco) = (loc(geo3d::add(depois, geo3d::mul(saida[ultimo], big))), loc(geo3d::add(antes, geo3d::mul(saida[0], big))));
        let ponta = p2[p2.len() - 1];
        let lp = [chapa::chain_segs(&p2), vec![fenkra_kernel::Seg2::Line(ponta, fim), fenkra_kernel::Seg2::Line(fim, comeco), fenkra_kernel::Seg2::Line(comeco, p2[0])]].concat();
        let alem = Shape::face(&fr, &[lp]).map_err(k)?.extrude(n, big, big).map_err(k)?;
        let parte = b.boolean(&alem, fenkra_kernel::Bool::Common).map_err(k)?;
        let fica = b.boolean(&alem, fenkra_kernel::Bool::Cut).map_err(k)?;
        if parte.is_null() || fica.is_null() || parte.mass().volume < 1e-6 || fica.mass().volume < 1e-6 {
            return Err("a linha de dobra em curva precisa dividir a chapa em duas partes".into());
        }
        // a parede tem a área da parte levantada: o desenvolvido fica igual
        let comprimento: f64 = cadeia.windows(2).map(|w| geo3d::dist(w[0], w[1])).sum();
        let parede = parte.mass().volume / (sp.t * comprimento.max(1e-6)) - cmp.ba;
        if parede <= 0. {
            return Err(format!("a parte além da curva não dá para a zona dobrada ({:.2} mm): afaste a linha da borda", cmp.ba));
        }
        self.bodies.insert(f.target, fresh_body(fica, f.id));
        self.curve_flange(f, cadeia, ang, r, parede, bd.down, None, 0)?;
        Ok(Some("dobra em curva: a parede tem largura constante ao longo da linha".into()))
    }
}

/// Índice do ponto da lista mais perto de `q`.
fn nearest_index(l: &[V3], q: V3) -> usize {
    (0..l.len()).min_by(|a, b| geo3d::dist(l[*a], q).total_cmp(&geo3d::dist(l[*b], q))).unwrap_or(0)
}

/// Trecho de `linha` que fica sobre a chapa, com as pontas achadas por bisseção na borda dela.
fn trecho_na_chapa(linha: &[V3], dentro: &dyn Fn(V3) -> bool) -> Option<Vec<V3>> {
    let i0 = linha.iter().position(|q| dentro(*q))?;
    let i1 = linha.iter().rposition(|q| dentro(*q))?;
    let borda = |mut sim: V3, mut nao: V3| {
        for _ in 0..BISSECOES {
            let m = geo3d::lerp(sim, nao, 0.5);
            if dentro(m) {
                sim = m;
            } else {
                nao = m;
            }
        }
        sim
    };
    let mut out = vec![];
    if i0 > 0 {
        out.push(borda(linha[i0], linha[i0 - 1]));
    }
    out.extend_from_slice(&linha[i0..=i1]);
    if i1 + 1 < linha.len() {
        out.push(borda(linha[i1], linha[i1 + 1]));
    }
    out.dedup_by(|a, b| geo3d::dist(*a, *b) < 1e-6);
    (out.len() >= 2).then_some(out)
}

/// Emenda polilinhas que se tocam pelas pontas numa só; `None` se alguma não encaixa.
fn encadeia(mut partes: Vec<Vec<V3>>) -> Option<Vec<V3>> {
    let mut out = partes.pop()?;
    while !partes.is_empty() {
        let (a, z) = (out[0], out[out.len() - 1]);
        let i = partes.iter().position(|p| [p[0], p[p.len() - 1]].iter().any(|q| geo3d::dist(*q, a) < SOBRE || geo3d::dist(*q, z) < SOBRE))?;
        let mut p = partes.swap_remove(i);
        if geo3d::dist(p[0], z) < SOBRE {
            out.extend(p.drain(1..));
        } else if geo3d::dist(p[p.len() - 1], z) < SOBRE {
            p.reverse();
            out.extend(p.drain(1..));
        } else if geo3d::dist(p[p.len() - 1], a) < SOBRE {
            p.pop();
            p.extend(out);
            out = p;
        } else {
            p.reverse();
            p.pop();
            p.extend(out);
            out = p;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emenda_partes_em_qualquer_sentido() {
        let c = encadeia(vec![vec![[1., 0., 0.], [2., 0., 0.]], vec![[3., 0., 0.], [2., 0., 0.]], vec![[0., 0., 0.], [1., 0., 0.]]]).unwrap();
        assert_eq!(c.len(), 4);
        assert!((geo3d::dist(c[0], c[3]) - 3.).abs() < 1e-9);
        assert!(encadeia(vec![vec![[0.; 3], [1., 0., 0.]], vec![[5., 0., 0.], [6., 0., 0.]]]).is_none());
    }

    #[test]
    fn trecho_corta_a_linha_na_borda() {
        let l = [[-10., 0., 0.], [1., 0., 0.], [2., 0., 0.], [30., 0., 0.]];
        let t = trecho_na_chapa(&l, &|q| q[0] >= 0. && q[0] <= 5.).unwrap();
        assert!(t[0][0].abs() < 1e-3 && (t[t.len() - 1][0] - 5.).abs() < 1e-3, "{t:?}");
        assert!(trecho_na_chapa(&l, &|_| false).is_none());
    }
}
