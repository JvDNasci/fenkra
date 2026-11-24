// SPDX-License-Identifier: LGPL-2.1-or-later
//! B580: seção de um corpo por um plano como contorno fechado (a "estação" da nervura de asa e da
//! caverna de fuselagem).
//!
//! Como funciona no kernel:
//!
//! - **Por que o corte de uma face só não serve.** O corte do plano com *uma* face de um sólido é uma
//!   curva aberta: a asa tem o bordo de fuga numa face própria, e a lateral cortada sozinha deixa a
//!   fresta. Fechar com uma linha à mão não acompanha a estação (a corda e o recuo mudam ao longo da
//!   envergadura) e a nervura quebrava ao mover o plano.
//! - **A seção do corpo.** O corpo inteiro é cortado pelo plano (booleana comum com uma face
//!   plana): sai a *face de seção*, cujo contorno externo é um fio fechado, e os furos dela são fios
//!   internos. O contorno mais longo é o perfil; ele já volta fechado, na ordem, e é refeito a cada
//!   recálculo, então mudar o plano ou a asa move o perfil junto.
//! - **Qual corpo.** O vínculo guarda só um ponto sobre o corpo escolhido. Como o corpo pode crescer
//!   ou encolher (a asa muda de corda), vale o corpo mais perto do ponto que o plano realmente
//!   corta, sem exigir que o ponto ainda esteja na superfície.
//! - **Pontos demais.** A malha do contorno vem com tolerância de 0,05 mm; trechos retos
//!   quase colineares são fundidos (tolerância de 0,05 mm), o que mantém a forma e tira os pontos
//!   que só pesariam no sketch.

use super::*;

/// Fura o contorno fechado `p` (sem repetir o primeiro ponto no fim): tira pontos que ficam a menos
/// de `tol` da reta entre os vizinhos que sobram (Douglas-Peucker), mantendo o fecho.
pub(crate) fn simplifica_fechado(p: &[V3], tol: f64) -> Vec<V3> {
    let n = p.len();
    if n < 8 {
        return p.to_vec();
    }
    // o ponto mais longe do primeiro divide o laço em duas cadeias abertas
    let meio = (1..n).max_by(|a, b| geo3d::dist(p[0], p[*a]).total_cmp(&geo3d::dist(p[0], p[*b]))).unwrap_or(n / 2);
    let mut manter = vec![false; n];
    manter[0] = true;
    manter[meio] = true;
    fn rec(p: &[V3], a: usize, b: usize, tol: f64, manter: &mut [bool]) {
        // a cadeia vai de `a` a `b`; o laço dá a volta quando b == p.len()
        let n = p.len();
        let (pa, pb) = (p[a % n], p[b % n]);
        let ab = geo3d::sub(pb, pa);
        let l2 = geo3d::dot(ab, ab).max(1e-18);
        let mut pior = (0., a);
        for i in a + 1..b {
            let ap = geo3d::sub(p[i % n], pa);
            let t = (geo3d::dot(ap, ab) / l2).clamp(0., 1.);
            let d = geo3d::dist(p[i % n], geo3d::add(pa, geo3d::mul(ab, t)));
            if d > pior.0 {
                pior = (d, i);
            }
        }
        if pior.0 > tol {
            manter[pior.1 % n] = true;
            rec(p, a, pior.1, tol, manter);
            rec(p, pior.1, b, tol, manter);
        }
    }
    rec(p, 0, meio, tol, &mut manter);
    rec(p, meio, n, tol, &mut manter);
    (0..n).filter(|i| manter[*i]).map(|i| p[i]).collect()
}

impl Ctx<'_> {
    /// Contorno fechado da seção do corpo mais perto de `ponto` pelo plano do sketch. O primeiro ponto
    /// se repete no fim, como o vínculo do sketch espera de um contorno fechado.
    pub(crate) fn secao_do_corpo(&self, ponto: V3, frame: &Frame) -> Result<Vec<V3>, String> {
        let metade = self.model_size() * 2.;
        // do corpo mais perto para o mais longe: o primeiro que o plano corta é o escolhido
        let mut candidatos: Vec<(f64, &Shape)> = self.bodies.values().filter(|(b, _, _)| !b.is_null()).map(|(b, _, _)| (b.surface_near(ponto).map(|(q, _)| geo3d::dist(q, ponto)).unwrap_or(f64::MAX), b)).collect();
        candidatos.sort_by(|a, b| a.0.total_cmp(&b.0));
        if candidatos.is_empty() {
            return Err("não há corpo para cortar: o corpo da seção não existe mais antes deste sketch".into());
        }
        for (_, corpo) in candidatos {
            let Ok(lacos) = corpo.section_loops(frame.origin, frame.normal, metade) else { continue };
            let comprimento = |l: &Vec<V3>| (0..l.len()).map(|i| geo3d::dist(l[i], l[(i + 1) % l.len()])).sum::<f64>();
            let Some(maior) = lacos.into_iter().filter(|l| l.len() >= 3).max_by(|a, b| comprimento(a).total_cmp(&comprimento(b))) else { continue };
            let mut p = simplifica_fechado(&maior, 0.05);
            let primeiro = p[0];
            p.push(primeiro);
            return Ok(p);
        }
        Err("o plano não corta nenhum sólido: leve o plano (a estação) para dentro do corpo".into())
    }
}
