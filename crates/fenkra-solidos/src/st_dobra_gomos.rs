// SPDX-License-Identifier: LGPL-2.1-or-later
//! Gomos do tubo redondo no corte simples: a geometria 3D (B326).
//!
//! Como funciona no kernel: a curva do tubo, de raio de centro `R` e deflexão total `θ`, é trocada por
//! `m` esquadrias de `φ = θ/m`. O caminho do eixo é um polígono circunscrito ao arco: as duas pernas
//! tocam o arco nos pontos de tangência, a `R·tan(θ/2)` do nó; o primeiro vértice fica `R·tan(φ/2)`
//! além do ponto de tangência, e os vértices seguintes distam `2R·tan(φ/2)` uns dos outros. Cada
//! vértice tem um plano de esquadria pela bissetriz das duas direções vizinhas.
//!
//! * as pernas são cortadas no plano do primeiro e do último vértice (ficam com a ponta recuada
//!   `R·tan(θ/2) − R·tan(φ/2)` do nó, que é o que a esquadria única de `m = 1` zera);
//! * as `m − 1` peças do meio são cilindros da seção entre dois planos de esquadria, cada um com
//!   `2R·tan(φ/2)` no eixo. Saem como peças separadas, que entram na lista de corte.
//!
//! Só tubo redondo: a seção é simétrica, então a peça do meio não precisa de orientação própria.

use super::*;
use crate::st_dobra_geo::NoDobra;
use crate::st_dobra_v::NoTratado;

/// Uma peça do meio: o sólido, o referencial (origem no início da peça, Z ao longo do eixo) e o
/// comprimento do eixo entre os dois planos de esquadria.
pub(crate) struct PecaGomo {
    pub shape: Shape,
    pub frame: Frame,
    pub comprimento: f64,
}

/// Recuo do primeiro vértice a partir do nó, ao longo da perna: `R·tan(θ/2) − R·tan(φ/2)`.
pub(crate) fn recuo_da_perna(raio: f64, theta_deg: f64, n: usize) -> f64 {
    let th = theta_deg.to_radians();
    raio * ((th / 2.).tan() - (th / n as f64 / 2.).tan())
}

/// Vértices do eixo no polígono dos gomos (`n` vértices: um por esquadria) e as direções dos `n + 1`
/// trechos (a perna que chega, os `n − 1` do meio e a que sai).
pub(crate) fn poligono(no: &NoDobra, n: usize) -> (Vec<V3>, Vec<V3>) {
    let th = no.theta.to_radians();
    let phi = th / n as f64;
    let meio = 2. * no.res.raio * (phi / 2.).tan();
    let dirs: Vec<V3> = (0..=n).map(|j| if j == n { no.t_out } else { geo3d::rotate(no.t_in, no.eixo, j as f64 * phi) }).collect();
    let a0 = geo3d::sub(no.p, geo3d::mul(no.t_in, recuo_da_perna(no.res.raio, no.theta, n)));
    let mut a = vec![a0];
    for d in dirs.iter().skip(1).take(n - 1) {
        a.push(geo3d::add(*a.last().unwrap(), geo3d::mul(*d, meio)));
    }
    (a, dirs)
}

fn k(e: KernelError) -> String {
    e.0
}

impl Ctx<'_> {
    /// Termina o nó de gomos: corta as duas pernas nos planos das pontas do polígono e devolve as
    /// peças do meio. `n` é o número de esquadrias (`gomos` do nó, maior que 1).
    pub(crate) fn tratar_gomos(&self, geo: &[MemberGeom], no: &NoDobra, n: usize, antes: &Shape, depois: &Shape) -> Result<(NoTratado, Vec<PecaGomo>), String> {
        if n < 2 {
            return Err("gomos pedem duas esquadrias ou mais".into());
        }
        if geo[no.antes].shift.iter().chain(geo[no.depois].shift.iter()).any(|s| s.abs() > 1e-9) {
            return Err("os gomos pedem a linha guia no centro da seção".into());
        }
        let raio = no.res.raio;
        let recuo = recuo_da_perna(raio, no.theta, n);
        for i in [no.antes, no.depois] {
            let len = geo3d::dist(geo[i].start, geo[i].end);
            if recuo > len + 1e-6 {
                return Err(format!("a perna tem {len:.1} mm e os gomos pedem {recuo:.1} mm antes do nó (raio menor, ou perna mais longa)"));
            }
        }
        let (a, dirs) = poligono(no, n);
        // o último vértice precisa cair no ponto simétrico da segunda perna
        let esperado = geo3d::add(no.p, geo3d::mul(no.t_out, recuo));
        if geo3d::dist(a[n - 1], esperado) > 0.05 {
            return Err(format!("o polígono dos gomos não fecha na segunda perna (erro {:.3} mm)", geo3d::dist(a[n - 1], esperado)));
        }
        let big = self.size_hint();
        let lado = |j: usize, sinal: f64| -> Result<Shape, String> {
            let nm = geo3d::unit(geo3d::sub(geo3d::mul(dirs[j], -1.), dirs[j + 1])).ok_or("as duas direções do nó são opostas")?;
            half_space(a[j], geo3d::mul(nm, sinal), big)
        };
        let antes2 = antes.boolean(&lado(0, 1.)?, fenkra_kernel::Bool::Common).map_err(k)?;
        let depois2 = depois.boolean(&lado(n - 1, -1.)?, fenkra_kernel::Bool::Common).map_err(k)?;
        let g = &geo[no.antes];
        let loops = structural::shifted(&g.loops, g.shift);
        let phi = no.theta.to_radians() / n as f64;
        let meio = 2. * raio * (phi / 2.).tan();
        let folga = 2. * g.w.max(g.h);
        let mut pecas = vec![];
        for j in 1..n {
            let dj = dirs[j];
            let xj = geo3d::rotate(no.x_in, no.eixo, j as f64 * phi);
            let s0 = geo3d::sub(a[j - 1], geo3d::mul(dj, folga));
            let face = Shape::face(&Frame { origin: s0, x: xj, normal: dj }, &loops).map_err(k)?;
            let prisma = face.extrude(dj, meio + 2. * folga, 0.).map_err(k)?;
            let peca = prisma.boolean(&lado(j - 1, -1.)?, fenkra_kernel::Bool::Common).map_err(k)?.boolean(&lado(j, 1.)?, fenkra_kernel::Bool::Common).map_err(k)?;
            pecas.push(PecaGomo { shape: peca, frame: Frame { origin: a[j - 1], x: xj, normal: dj }, comprimento: meio });
        }
        Ok((NoTratado { antes: antes2, depois: depois2, extras: vec![] }, pecas))
    }
}
