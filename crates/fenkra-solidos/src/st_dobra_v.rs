// SPDX-License-Identifier: LGPL-2.1-or-later
//! Dobra de tubo por rasgos em V e canto com gabarito: a geometria 3D do tubo **fechado** (B328,
//! B333).
//!
//! Como funciona no kernel: as pernas chegam ao nó já alongadas (e, no V comum e no canto com
//! gabarito, cortadas em esquadria pela bissetriz, como no corte simples). Este módulo termina o
//! nó:
//!
//! * **V comum e corte + dobradeira**: com o V fechado, as paredes laterais e a interna se tocam na
//!   esquadria; só a placa externa (a dobradiça) se curva. Tira-se da esquadria o "esporão" que
//!   passa do raio externo da dobradiça (`ri + t`) e funde-se a placa em arco, girando a seção da
//!   placa por `θ` em torno do centro da curva.
//! * **V múltiplo**: `n` esquadrias de `φ = θ/n` ao longo de um polígono de articulações; as `n − 1`
//!   peças do meio são prismas da seção entre dois planos de esquadria. Os vértices ficam na face
//!   interna da placa, com canto vivo (a pesquisa não dá raio de dobradiça para o múltiplo).
//! * **Canto com gabarito**: a placa externa vira arco (raio externo `Ro`), cada lateral mantém um
//!   setor de raio `Ro − t` com ápice no centro da curva, e a parede interna segue até a bissetriz
//!   (as laterais, fora do setor, terminam no início do arco).
//!
//! Todos os números vêm de `docs/pesquisa/dobra-de-tubos.md` (seções 7 e 8), derivados sem
//! protótipo: o painel diz "a validar em peça real".

use super::*;
use crate::st_dobra_geo::NoDobra;
use fenkra_kernel::Seg2;

/// Plano da dobra visto de uma perna: `x` ao longo da perna em direção ao nó, `y` da face externa
/// para o centro da curva, `w` ao longo do eixo da dobra (`ux × uy = uz`).
#[derive(Clone, Copy)]
struct Quadro {
    op: V3,
    ux: V3,
    uy: V3,
    uz: V3,
}

impl Quadro {
    fn p(&self, x: f64, y: f64, w: f64) -> V3 {
        geo3d::add(geo3d::add(geo3d::add(self.op, geo3d::mul(self.ux, x)), geo3d::mul(self.uy, y)), geo3d::mul(self.uz, w))
    }
}

/// O nó depois do tratamento: as duas pernas recortadas e os corpos a fundir.
pub(crate) struct NoTratado {
    pub antes: Shape,
    pub depois: Shape,
    pub extras: Vec<Shape>,
}

fn k(e: KernelError) -> String {
    e.0
}

fn linhas(pts: &[[f64; 2]]) -> Vec<Seg2> {
    (0..pts.len()).map(|i| Seg2::Line(pts[i], pts[(i + 1) % pts.len()])).collect()
}

/// Face plana no plano `x` da perna; as coordenadas do laço são (u, v) = (−w, y).
fn face_em(q: &Quadro, x: f64, laco: Vec<Seg2>) -> Result<Shape, String> {
    let fr = Frame { origin: q.p(x, 0., 0.), x: geo3d::mul(q.uz, -1.), normal: q.ux };
    Shape::face(&fr, &[laco]).map_err(k)
}

/// Caixa alinhada ao quadro: de `x0` a `x1`, `y0` a `y1`, `w0` a `w1`.
fn caixa(q: &Quadro, x: (f64, f64), y: (f64, f64), w: (f64, f64)) -> Result<Shape, String> {
    Shape::box_tool(q.p(x.0, y.0, w.0), q.ux, q.uz, [x.1 - x.0, y.1 - y.0, w.1 - w.0]).map_err(k)
}

/// Plano de esquadria entre duas direções de percurso: ponto e normal que fica do lado da peça que
/// chega (a que vem com `d_in`).
fn plano_de_junta(a: V3, d_in: V3, d_out: V3) -> Result<(V3, V3), String> {
    let n = geo3d::unit(geo3d::sub(geo3d::mul(d_in, -1.), d_out)).ok_or("as duas direções do nó são opostas")?;
    Ok((a, n))
}

impl Ctx<'_> {
    /// Termina o nó `no` de um tubo com rasgo em V ou canto com gabarito. `antes` e `depois` são
    /// os sólidos das duas pernas.
    pub(crate) fn tratar_no_v(&self, geo: &[MemberGeom], no: &NoDobra, antes: &Shape, depois: &Shape) -> Result<NoTratado, String> {
        let r = &no.res;
        let (h, b, t) = (r.h_plano, r.b_fora, r.parede);
        let al = geo3d::dot(no.x_in, no.eixo).abs();
        if !(al > 0.999 || al < 0.02) {
            return Err("a seção não está alinhada ao plano da dobra: os rasgos em V pedem a seção alinhada".into());
        }
        if geo[no.antes].shift.iter().chain(geo[no.depois].shift.iter()).any(|s| s.abs() > 1e-9) {
            return Err("os rasgos em V pedem a linha guia no centro da seção".into());
        }
        let n1 = geo3d::unit(fenkra_kernel::cross(no.eixo, no.t_in)).ok_or("dobra sem plano")?;
        let n2 = geo3d::unit(fenkra_kernel::cross(no.eixo, no.t_out)).ok_or("dobra sem plano")?;
        let q1 = Quadro { op: geo3d::sub(no.p, geo3d::mul(n1, h / 2.)), ux: no.t_in, uy: n1, uz: no.eixo };
        let q2 = Quadro { op: geo3d::sub(no.p, geo3d::mul(n2, h / 2.)), ux: geo3d::mul(no.t_out, -1.), uy: n2, uz: geo3d::mul(no.eixo, -1.) };
        let th = no.theta.to_radians();
        let hf = th / 2.;
        let big = self.size_hint();
        match no.bend.mode {
            BendMode::VComRaio => self.canto_com_gabarito(no, &q1, &q2, (h, b, t), antes, depois, big),
            BendMode::VComum | BendMode::CorteDobradeira if r.rasgos <= 1 => {
                // dobradiça: raio interno ri, raio externo ri + t
                let ro = r.raio + t;
                let x0 = (h / 2. - ro) * hf.tan();
                let c = q1.p(x0, ro, 0.);
                let esporao = self.esporao(&q1, th, (h, b), ro)?;
                let banda = face_em(&q1, x0, linhas(&[[-b / 2., 0.], [b / 2., 0.], [b / 2., t], [-b / 2., t]]))?.revolve(c, q1.uz, no.theta, 0.).map_err(k)?;
                Ok(NoTratado {
                    antes: antes.boolean(&esporao, fenkra_kernel::Bool::Cut).map_err(k)?,
                    depois: depois.boolean(&esporao, fenkra_kernel::Bool::Cut).map_err(k)?,
                    extras: vec![banda],
                })
            }
            BendMode::VComum | BendMode::CorteDobradeira => {
                let n = r.rasgos;
                let phi = th / n as f64;
                let p = r.passo_v.ok_or("V múltiplo sem passo")?;
                let rv = p / (2. * (phi / 2.).sin());
                let s = rv * (((n - 1) as f64) * phi / 2.).sin() / hf.cos();
                let x_a0 = (h / 2. - t) * hf.tan() - s;
                // direções de cada peça e vértices da face interna da placa
                let dirs: Vec<V3> = (0..=n).map(|j| if j == n { no.t_out } else { geo3d::rotate(no.t_in, no.eixo, j as f64 * phi) }).collect();
                let mut a: Vec<V3> = vec![q1.p(x_a0, t, 0.)];
                for d in dirs.iter().skip(1).take(n - 1) {
                    a.push(geo3d::add(*a.last().unwrap(), geo3d::mul(*d, p)));
                }
                let esperado = q2.p(x_a0, t, 0.);
                if geo3d::dist(a[n - 1], esperado) > 0.05 {
                    return Err(format!("o polígono do V múltiplo não fecha na segunda perna (erro {:.3} mm)", geo3d::dist(a[n - 1], esperado)));
                }
                let juntas: Vec<(V3, V3)> = (0..n).map(|j| plano_de_junta(a[j], dirs[j], dirs[j + 1])).collect::<Result<_, _>>()?;
                let lado = |(pt, nm): (V3, V3), sinal: f64| half_space(pt, geo3d::mul(nm, sinal), big);
                let antes2 = antes.boolean(&lado(juntas[0], 1.)?, fenkra_kernel::Bool::Common).map_err(k)?;
                let depois2 = depois.boolean(&lado(juntas[n - 1], -1.)?, fenkra_kernel::Bool::Common).map_err(k)?;
                // peças do meio: a seção girada ao longo do polígono
                let g = &geo[no.antes];
                let loops = structural::shifted(&g.loops, g.shift);
                let m = 2. * h.max(b);
                let mut extras = vec![];
                for j in 1..n {
                    let ang = j as f64 * phi;
                    let nj = geo3d::rotate(n1, no.eixo, ang);
                    let xj = geo3d::rotate(no.x_in, no.eixo, ang);
                    let dj = dirs[j];
                    let s0 = geo3d::sub(geo3d::add(a[j - 1], geo3d::mul(nj, h / 2. - t)), geo3d::mul(dj, m));
                    let face = Shape::face(&Frame { origin: s0, x: xj, normal: dj }, &loops).map_err(k)?;
                    let prisma = face.extrude(dj, p + 2. * m, 0.).map_err(k)?;
                    let peca = prisma
                        .boolean(&lado(juntas[j - 1], -1.)?, fenkra_kernel::Bool::Common)
                        .map_err(k)?
                        .boolean(&lado(juntas[j], 1.)?, fenkra_kernel::Bool::Common)
                        .map_err(k)?;
                    extras.push(peca);
                }
                Ok(NoTratado { antes: antes2, depois: depois2, extras })
            }
            _ => Err("este modo de dobra não tem rasgo em V".into()),
        }
    }

    /// O que sobra da esquadria fora do raio externo `ro` da placa curvada, entre os planos de
    /// início e fim do arco.
    fn esporao(&self, q1: &Quadro, th: f64, (h, b): (f64, f64), ro: f64) -> Result<Shape, String> {
        let hf = th / 2.;
        let x0 = (h / 2. - ro) * hf.tan();
        let xo = (h / 2.) * hf.tan();
        let t1 = [x0, 0.];
        let o = [xo, 0.];
        let tg = ro * hf.tan();
        let t2 = [xo + tg * th.cos(), tg * th.sin()];
        let meio_do_arco = [x0 + ro * hf.sin(), ro - ro * hf.cos()];
        let laco = vec![Seg2::Line(t1, o), Seg2::Line(o, t2), Seg2::Arc(t2, meio_do_arco, t1)];
        let fr = Frame { origin: q1.op, x: q1.ux, normal: q1.uz };
        Shape::face(&fr, &[laco]).map_err(k)?.extrude(q1.uz, b / 2. + 2., b / 2. + 2.).map_err(k)
    }

    /// Canto com gabarito (7.3 da pesquisa): placa em arco, setores nas laterais, parede interna
    /// até a bissetriz.
    #[allow(clippy::too_many_arguments)]
    fn canto_com_gabarito(&self, no: &NoDobra, q1: &Quadro, q2: &Quadro, (h, b, t): (f64, f64, f64), antes: &Shape, depois: &Shape, big: f64) -> Result<NoTratado, String> {
        let r = &no.res;
        let ro = r.raio;
        let hf = no.theta.to_radians() / 2.;
        let rf = ro - t;
        let x0 = (h / 2. - ro) * hf.tan();
        let c = q1.p(x0, ro, 0.);
        let c2 = q2.p(x0, ro, 0.);
        if geo3d::dist(c, c2) > 0.01 {
            return Err("o centro da curva não coincide nas duas pernas".into());
        }
        // 1. tira das pernas o que a placa curva e os setores substituem: a placa e as laterais
        // depois do início do arco (a parede interna fica, até a esquadria)
        let comprimento = 3. * (h + ro) + 50.;
        let corta = |q: &Quadro, s: &Shape| -> Result<Shape, String> {
            let mut s = s.boolean(&caixa(q, (x0, x0 + comprimento), (-1., t), (-b / 2. - 1., b / 2. + 1.))?, fenkra_kernel::Bool::Cut).map_err(k)?;
            for (w0, w1) in [(-b / 2. - 1., -b / 2. + t), (b / 2. - t, b / 2. + 1.)] {
                s = s.boolean(&caixa(q, (x0, x0 + comprimento), (t, h + 1.), (w0, w1))?, fenkra_kernel::Bool::Cut).map_err(k)?;
            }
            Ok(s)
        };
        let (antes2, depois2) = (corta(q1, antes)?, corta(q2, depois)?);
        // 2. placa em arco e setores: o perfil em U girado em torno do centro da curva
        let topo = h.min(ro);
        let banda_laco = linhas(&[[-b / 2., 0.], [b / 2., 0.], [b / 2., t], [-b / 2., t]]);
        let tem_nervura = topo > t + 1e-6;
        let com_folga = r.folga_raiz > 1e-9;
        let mut extras = vec![];
        let nervuras_laco = |sinal: f64| linhas(&[[sinal * (b / 2. - t), t], [sinal * (b / 2.), t], [sinal * (b / 2.), topo], [sinal * (b / 2. - t), topo]]);
        let gira = |laco: Vec<Seg2>| -> Result<Shape, String> { face_em(q1, x0, laco)?.revolve(c, q1.uz, no.theta, 0.).map_err(k) };
        if !tem_nervura {
            extras.push(gira(banda_laco)?);
        } else if !com_folga {
            let u = [[-b / 2., 0.], [b / 2., 0.], [b / 2., topo], [b / 2. - t, topo], [b / 2. - t, t], [-b / 2. + t, t], [-b / 2. + t, topo], [-b / 2., topo]];
            extras.push(self.limitar_ao_interior(gira(linhas(&u))?, q1, q2, h, ro, big)?);
        } else {
            let mut nervuras = Vec::new();
            for sinal in [-1., 1.] {
                nervuras.push(gira(nervuras_laco(sinal))?);
            }
            // a folga de raiz é uma faixa de largura e centrada na bissetriz, que só corta as nervuras
            let bis = geo3d::add(geo3d::mul(q1.ux, hf.sin()), geo3d::mul(q1.uy, -hf.cos()));
            let perp = fenkra_kernel::cross(q1.uz, bis);
            let origem = geo3d::sub(geo3d::sub(geo3d::sub(c, geo3d::mul(bis, 1.)), geo3d::mul(perp, r.folga_raiz / 2.)), geo3d::mul(q1.uz, b / 2. + 1.));
            let faixa = Shape::box_tool(origem, bis, q1.uz, [rf + 3., r.folga_raiz, b + 2.]).map_err(k)?;
            extras.push(gira(banda_laco)?);
            for n in nervuras {
                let cortada = n.boolean(&faixa, fenkra_kernel::Bool::Cut).map_err(k)?;
                extras.push(self.limitar_ao_interior(cortada, q1, q2, h, ro, big)?);
            }
        }
        Ok(NoTratado { antes: antes2, depois: depois2, extras })
    }

    /// Com `Ro > H` as nervuras giradas passam um pouco da face interna das pernas (a revolução
    /// dá uma coroa, e o canto de dentro é em V): tira o que passa das duas faces internas juntas.
    fn limitar_ao_interior(&self, s: Shape, q1: &Quadro, q2: &Quadro, h: f64, ro: f64, big: f64) -> Result<Shape, String> {
        if ro <= h + 1e-6 {
            return Ok(s);
        }
        // o vazio do canto de dentro é o que passa das duas faces internas ao mesmo tempo
        let vazio = half_space(q1.p(0., h, 0.), q1.uy, big)?.boolean(&half_space(q2.p(0., h, 0.), q2.uy, big)?, fenkra_kernel::Bool::Common).map_err(k)?;
        let s = s.boolean(&vazio, fenkra_kernel::Bool::Cut).map_err(k)?;
        Ok(s)
    }
}
