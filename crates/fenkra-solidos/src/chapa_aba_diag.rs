// SPDX-License-Identifier: LGPL-2.1-or-later
//! B566: validação leve da aba por superfície, antes do cálculo completo.
//!
//! Como funciona no kernel: a aba por superfície sai de uma curva no plano da web (a OML, interseção
//! do suporte com o plano da face base). A dobra é uma varredura de seção fixa (arco de raio interno
//! `r`, externo `r + t`, mais a parede) ao longo dessa curva, e a planificada é a faixa que a curva
//! varre ao andar `ba + comprimento` para o lado da aba. As duas viram a mesma pergunta geométrica: o
//! que acontece quando uma curva é deslocada de uma distância `d` no plano?
//!
//! - No lado **convexo** (o centro de curvatura fica do lado da web), o deslocamento abre em leque e
//!   nunca se cruza; o material só precisa esticar um pouco.
//! - No lado **côncavo** (o centro de curvatura fica do lado da aba), o deslocamento fecha: com `d`
//!   igual ao raio de curvatura `R` ele vira um ponto, e com `d > R` forma um bico que se cruza (o
//!   enrugamento de uma superfície que não se desenvolve). Por isso a conta é um número só: o menor
//!   raio de curvatura côncavo `R` da OML.
//!
//! Daí as duas perguntas da validação: a **dobra cabe** quando `R > r + t` (a seção da dobra não se
//! cruza ao varrer) e a aba **planifica** quando `R > ba + comprimento` (a faixa plana não se cruza).
//! A conta usa só a polilinha da OML, em milissegundos, e dá o trecho exato onde falha.

use crate::geo3d::{self, V3};

/// Resultado da validação de uma aba sobre uma OML.
#[derive(Clone, Debug, PartialEq)]
pub struct DiagAba {
    /// menor raio de curvatura côncavo (lado da aba) da OML e o ponto onde ocorre; `None` se não há
    /// trecho côncavo
    pub concavo: Option<(f64, V3)>,
    /// raio mínimo da dobra que ainda cabe (`r + t`)
    pub raio_dobra: f64,
    /// desenvolvimento da aba (`ba + comprimento`): o quanto a planificada avança para fora
    pub desenvolvimento: f64,
}

impl DiagAba {
    /// A seção da dobra varre sem se cruzar?
    pub fn cabe(&self) -> bool {
        self.concavo.is_none_or(|(r, _)| r > self.raio_dobra * 1.01)
    }

    /// A faixa planificada não se cruza?
    pub fn planifica(&self) -> bool {
        self.concavo.is_none_or(|(r, _)| r > self.desenvolvimento * 1.01)
    }

    /// Maior comprimento de aba que ainda planifica nesta OML (`None` sem trecho côncavo).
    pub fn comprimento_maximo(&self, ba: f64) -> Option<f64> {
        self.concavo.map(|(r, _)| (r / 1.01 - ba).max(0.))
    }

    /// Texto claro do que não vale, ou `None` se está tudo certo.
    pub fn aviso(&self, ba: f64) -> Option<String> {
        let (r, p) = self.concavo?;
        if !self.cabe() {
            return Some(format!(
                "a dobra não cabe perto de ({:.1}, {:.1}, {:.1}): a curva fecha com raio {:.1} mm, menor que o raio externo da dobra ({:.1} mm); aumente o raio da curva ou diminua o raio de dobra",
                p[0], p[1], p[2], r, self.raio_dobra
            ));
        }
        if !self.planifica() {
            return Some(format!(
                "a aba não planifica perto de ({:.1}, {:.1}, {:.1}): a curva fecha com raio {:.1} mm, menor que o desenvolvimento da aba ({:.1} mm) e a faixa plana se cruza; use comprimento até {:.1} mm ou o apoio por aproximação",
                p[0], p[1], p[2], r, self.desenvolvimento, self.comprimento_maximo(ba).unwrap_or(0.)
            ));
        }
        None
    }
}

/// Raio do círculo por três pontos e o vetor do ponto do meio até o centro; `None` se colineares.
fn circulo(a: V3, b: V3, c: V3) -> Option<(f64, V3)> {
    let (ab, ac) = (geo3d::sub(b, a), geo3d::sub(c, a));
    let cr = fenkra_kernel::cross(ab, ac);
    let area2 = geo3d::norm(cr);
    let (la, lb, lc) = (geo3d::norm(ab), geo3d::norm(geo3d::sub(c, b)), geo3d::norm(ac));
    if area2 < 1e-9 * (la * lb * lc).max(1e-12) {
        return None;
    }
    let r = la * lb * lc / (2. * area2);
    // centro: a + (|ac|² (ab × ac) × ab + |ab|² ac × (ab × ac)) / (2 |ab × ac|²)
    let t1 = geo3d::mul(fenkra_kernel::cross(cr, ab), geo3d::dot(ac, ac));
    let t2 = geo3d::mul(fenkra_kernel::cross(ac, cr), geo3d::dot(ab, ab));
    let centro = geo3d::add(a, geo3d::mul(geo3d::add(t1, t2), 1. / (2. * area2 * area2)));
    Some((r, geo3d::sub(centro, b)))
}

/// Valida uma aba sobre a OML `pts` (polilinha no plano de normal `n`; `fora` aponta da web para o
/// lado da aba). `r` e `t` são o raio de dobra e a espessura; `desenvolvimento` é `ba + comprimento`.
pub fn diagnosticar(pts: &[V3], n: V3, fora: V3, r: f64, t: f64, desenvolvimento: f64) -> DiagAba {
    let mut diag = DiagAba { concavo: None, raio_dobra: r + t, desenvolvimento };
    if pts.len() < 3 {
        return diag;
    }
    let mut s = vec![0.];
    for w in pts.windows(2) {
        s.push(s.last().unwrap() + geo3d::dist(w[0], w[1]));
    }
    let total = *s.last().unwrap();
    // janela de três pontos: longa o bastante para não medir o ruído da amostragem (a polilinha vem de
    // uma interseção subdividida), curta o bastante para pegar o raio de uma curva fechada
    let passo = total / (pts.len() - 1) as f64;
    // B566: o piso de 0,75 mm vale até 1/20 da linha (numa peça de 10 mm a janela era 7 % dela)
    let h = (passo * 3.).max(0.75f64.min(total / 20.)).min(total / 4.);
    let mut menor: Option<(f64, V3)> = None;
    for i in 1..pts.len() - 1 {
        // B566: perto das pontas a janela fica de um lado só (até a ponta), com pelo menos um terço dela:
        // antes as pontas ficavam de fora, e a curva mais fechada da asa (perto do canto) não era vista
        let Some(j) = (0..i).rev().find(|j| s[i] - s[*j] >= h).or((s[i] >= h / 3.).then_some(0)) else { continue };
        let Some(k) = (i + 1..pts.len()).find(|k| s[*k] - s[i] >= h).or((total - s[i] >= h / 3.).then_some(pts.len() - 1)) else { continue };
        let Some((rc, para_centro)) = circulo(pts[j], pts[i], pts[k]) else { continue };
        // o centro no lado da aba (e no plano da OML): curva côncava para a aba
        let no_plano = geo3d::sub(para_centro, geo3d::mul(n, geo3d::dot(n, para_centro)));
        if geo3d::dot(no_plano, fora) > 0. && menor.is_none_or(|(m, _)| rc < m) {
            menor = Some((rc, pts[i]));
        }
    }
    diag.concavo = menor;
    diag
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Arco de círculo de raio `r` no plano z = 0, de `a0` a `a1` (graus).
    fn arco(r: f64, a0: f64, a1: f64) -> Vec<V3> {
        (0..=100).map(|i| (a0 + (a1 - a0) * i as f64 / 100.).to_radians()).map(|a| [r * a.cos(), r * a.sin(), 0.]).collect()
    }

    #[test]
    fn arco_com_a_aba_para_dentro_e_concavo_e_mede_o_raio() {
        let pts = arco(30., 0., 90.);
        // a aba sobe para o centro do círculo: lado côncavo
        let d = diagnosticar(&pts, [0., 0., 1.], [-1., -1., 0.], 2., 2., 20.);
        let (r, _) = d.concavo.expect("trecho côncavo");
        assert!((r - 30.).abs() < 0.3, "raio medido {r}");
        assert!(d.cabe() && d.planifica(), "20 mm de desenvolvimento cabem em R30");
        let d = diagnosticar(&pts, [0., 0., 1.], [-1., -1., 0.], 2., 2., 35.);
        assert!(d.cabe() && !d.planifica(), "35 mm não planifica em R30");
        assert!(d.aviso(5.).unwrap().contains("não planifica"));
        assert!((d.comprimento_maximo(5.).unwrap() - (30. / 1.01 - 5.)).abs() < 0.5);
    }

    #[test]
    fn arco_com_a_aba_para_fora_e_convexo_e_sempre_planifica() {
        let pts = arco(30., 0., 90.);
        let d = diagnosticar(&pts, [0., 0., 1.], [1., 1., 0.], 2., 2., 500.);
        assert!(d.concavo.is_none() && d.planifica() && d.cabe() && d.aviso(5.).is_none());
    }

    #[test]
    fn reta_e_curva_curta_nao_geram_aviso() {
        let reta: Vec<V3> = (0..=50).map(|i| [i as f64, 0., 0.]).collect();
        assert!(diagnosticar(&reta, [0., 0., 1.], [0., 1., 0.], 2., 2., 20.).aviso(5.).is_none());
        assert!(diagnosticar(&reta[..2], [0., 0., 1.], [0., 1., 0.], 2., 2., 20.).aviso(5.).is_none());
    }

    #[test]
    fn raio_menor_que_a_dobra_nao_cabe() {
        let pts = arco(3., 0., 180.);
        let d = diagnosticar(&pts, [0., 0., 1.], [-1., 0., 0.], 2., 2., 10.);
        assert!(!d.cabe());
        assert!(d.aviso(5.).unwrap().contains("não cabe"));
    }
}
