// SPDX-License-Identifier: LGPL-2.1-or-later
//! Corte de encaixe normal entre membros (B155): a curva que o laser ou o plasma percorre na ponta
//! de um tubo para ele encostar no tubo vizinho, mais o contorno plano dessa curva (tubo aberto).
//!
//! # Como funciona
//!
//! O membro cortado tem duas peles: a face externa e a face interna da parede. Cada ponto do
//! perímetro tem, então, dois raios paralelos ao eixo do membro, um por pele. Cada raio vai de
//! dentro do membro até o hospedeiro (o tubo vizinho, tratado como sólido convexo: cilindro ou
//! caixa, já engordado pela folga de solda) e sai dele no parâmetro `λ`, medido ao longo do eixo a
//! partir do nó do esqueleto. O membro só existe para `λ` maior que o corte.
//!
//! * Um laser de 3 eixos corta **perpendicular à parede**: a curva é uma só, `λ(s)`, valendo para a
//!   espessura inteira. Se ela passa pela pele que toca primeiro (a de maior `λ`), a outra pele fica
//!   com uma abertura de raiz `|λ_ext − λ_int|`; se passa por qualquer outro lugar, uma das peles
//!   invade o hospedeiro. O modo automático escolhe, ponto a ponto, a pele de maior `λ`: não invade
//!   nunca e dá a menor abertura possível. A troca de pele acontece onde a abertura é zero, por isso
//!   a curva não tem degrau (não precisa de histerese). Os modos interna, externa e média existem
//!   para quem quer decidir; eles podem invadir, e a invasão é medida e reportada.
//! * A abertura é medida na normal do hospedeiro (`axial × |cos|` do ângulo entre o eixo e a normal),
//!   que é o espaço real que a solda tem de preencher. Acima do orçamento (abertura máxima
//!   admissível) o trecho é marcado: só um chanfro resolve.
//! * Com chanfro (cabeçote que inclina), a face de corte acompanha o hospedeiro de uma pele à outra:
//!   abertura e invasão são zero e o ângulo do chanfro em cada ponto é `atan(Δλ / parede)`. Esse
//!   mesmo ângulo, nos cortes perpendiculares, vale como o chanfro que faltou (`bisel_ideal`).
//!
//! O contorno plano desenrola a pele externa: X é o `λ` do ponto (distância do nó) e Y é o
//! perímetro externo, que no tubo redondo é `R·θ` e no retangular a soma dos lados. O
//! desenvolvimento preserva comprimento (a pele externa é um cilindro ou uma caixa), então o
//! comprimento de corte do desenho é o comprimento da curva 3D.
//!
//! Limites desta primeira versão: seções redondas, quadradas, retangulares e barras (sem raio de
//! canto nem ovalização); hospedeiro convexo; tubo dobrado ainda não leva o contorno no seu
//! planificado.

use crate::geo3d;
use crate::st_dobra_corte::{Camada, Desenho};

type V3 = [f64; 3];

/// Seção transversal suportada. `t` é a parede (0 = barra maciça).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Secao {
    Redonda { d: f64, t: f64 },
    Retangular { b: f64, h: f64, t: f64 },
}

impl Secao {
    pub fn parede(&self) -> f64 {
        match *self {
            Secao::Redonda { t, .. } | Secao::Retangular { t, .. } => t,
        }
    }
    /// Perímetro da pele externa.
    pub fn perimetro(&self) -> f64 {
        match *self {
            Secao::Redonda { d, .. } => std::f64::consts::PI * d,
            Secao::Retangular { b, h, .. } => 2. * (b + h),
        }
    }
    /// Maior meia-dimensão (para o recuo do hospedeiro).
    pub fn meia(&self) -> f64 {
        match *self {
            Secao::Redonda { d, .. } => d / 2.,
            Secao::Retangular { b, h, .. } => b.max(h) / 2.,
        }
    }
}

/// Um membro visto da junta: seção, onde o eixo cruza o plano da seção no canto e os eixos.
#[derive(Clone, Debug)]
pub struct Membro {
    pub secao: Secao,
    /// ponto do eixo no canto (nó do esqueleto)
    pub ponto: V3,
    /// eixos da seção (o mesmo referencial do membro, não depende da ponta)
    pub x: V3,
    pub y: V3,
    /// do canto para dentro do membro
    pub para_dentro: V3,
    /// centro da seção em (x, y), quando a ancoragem desloca a seção da linha guia
    pub centro: [f64; 2],
    /// distância do canto à outra ponta
    pub comprimento: f64,
}

/// Como escolher a pele (parede) que a curva acompanha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ModoParede {
    /// ponto a ponto, a pele que toca primeiro: nunca invade e dá a menor abertura
    #[default]
    Automatico,
    Interna,
    Externa,
    Media,
}

impl ModoParede {
    pub const TODOS: [ModoParede; 4] = [ModoParede::Automatico, ModoParede::Externa, ModoParede::Interna, ModoParede::Media];
    pub fn nome(self) -> &'static str {
        match self {
            ModoParede::Automatico => "Automático",
            ModoParede::Interna => "Parede interna",
            ModoParede::Externa => "Parede externa",
            ModoParede::Media => "Média das paredes",
        }
    }
}

/// Parâmetros do corte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opcoes {
    pub modo: ModoParede,
    /// abertura de raiz máxima admissível, em mm (na normal do hospedeiro)
    pub orcamento: f64,
    /// corte perpendicular à parede (laser de 3 eixos); desligado = com chanfro, a face acompanha o
    /// hospedeiro
    pub perpendicular: bool,
    /// folga de solda: o hospedeiro engorda por ela
    pub folga: f64,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes { modo: ModoParede::Automatico, orcamento: 1.5, perpendicular: true, folga: 0. }
    }
}

/// Pele em vigor num ponto da curva.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parede {
    Externa,
    Interna,
    Media,
    /// o raio não encontra o hospedeiro: a ponta fica no plano do nó
    Livre,
}

/// Um ponto da curva de corte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PontoCorte {
    /// posição no perímetro externo, em mm (0 = meio da face +x, sentido anti-horário)
    pub s: f64,
    /// distância do nó, ao longo do eixo, onde a curva corta
    pub lambda: f64,
    pub parede: Parede,
    /// abertura de raiz na normal do hospedeiro
    pub folga_raiz: f64,
    /// quanto a outra pele invade o hospedeiro (0 nos modos que não invadem)
    pub invasao: f64,
    /// chanfro usado, em graus (0 = perpendicular)
    pub bisel: f64,
    /// chanfro que zeraria a abertura, em graus
    pub bisel_ideal: f64,
    pub excede: bool,
    /// ponto 3D da pele externa na curva
    pub p3: V3,
    pub lambda_ext: f64,
    pub lambda_int: f64,
}

/// A curva de corte de uma ponta.
#[derive(Clone, Debug)]
pub struct CorteEncaixe {
    pub secao: Secao,
    pub opc: Opcoes,
    pub pontos: Vec<PontoCorte>,
    pub perimetro: f64,
}

impl CorteEncaixe {
    pub fn lambda_min(&self) -> f64 {
        self.pontos.iter().map(|p| p.lambda).fold(f64::MAX, f64::min)
    }
    pub fn lambda_max(&self) -> f64 {
        self.pontos.iter().map(|p| p.lambda).fold(f64::MIN, f64::max)
    }
    pub fn folga_max(&self) -> f64 {
        self.pontos.iter().map(|p| p.folga_raiz).fold(0., f64::max)
    }
    pub fn invasao_max(&self) -> f64 {
        self.pontos.iter().map(|p| p.invasao).fold(0., f64::max)
    }
    pub fn bisel_max(&self) -> f64 {
        self.pontos.iter().map(|p| p.bisel.abs().max(if self.opc.perpendicular { 0. } else { p.bisel_ideal.abs() })).fold(0., f64::max)
    }
    /// Trechos (de s0 a s1) acima do orçamento de folga.
    pub fn trechos_acima(&self) -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = vec![];
        let mut ini: Option<f64> = None;
        for p in &self.pontos {
            match (p.excede, ini) {
                (true, None) => ini = Some(p.s),
                (false, Some(a)) => {
                    out.push((a, p.s));
                    ini = None;
                }
                _ => {}
            }
        }
        if let Some(a) = ini {
            out.push((a, self.perimetro));
        }
        out
    }
    /// Comprimento da curva desenrolada (é o de corte).
    pub fn comprimento_corte(&self) -> f64 {
        self.pontos.windows(2).map(|w| (w[1].s - w[0].s).hypot(w[1].lambda - w[0].lambda)).sum()
    }
    /// Comprimento da mesma curva medido no 3D, sobre a pele externa.
    pub fn comprimento_3d(&self) -> f64 {
        self.pontos.windows(2).map(|w| geo3d::dist(w[0].p3, w[1].p3)).sum()
    }
    /// O ponto da curva mais próximo de `s` (para o realce ao passar o mouse).
    pub fn ponto_em(&self, s: f64) -> Option<&PontoCorte> {
        let p = self.perimetro;
        let s = s.rem_euclid(p);
        self.pontos.iter().min_by(|a, b| {
            let d = |q: &PontoCorte| {
                let d = (q.s - s).abs();
                d.min(p - d)
            };
            d(a).total_cmp(&d(b))
        })
    }
    /// Frações de parede em vigor: (externa, interna, média, livre), somando 1.
    pub fn fracao_por_parede(&self) -> [f64; 4] {
        let n = self.pontos.len().max(1) as f64;
        let c = |w: Parede| self.pontos.iter().filter(|p| p.parede == w).count() as f64 / n;
        [c(Parede::Externa), c(Parede::Interna), c(Parede::Media), c(Parede::Livre)]
    }
}

// ----------------------------------------------------------------------------- amostragem

/// Par de pontos (pele externa, pele interna) na seção, com a posição `s` no perímetro externo.
fn amostras(sec: Secao) -> Vec<(f64, [f64; 2], [f64; 2])> {
    match sec {
        Secao::Redonda { d, t } => {
            let (ro, ri) = (d / 2., (d / 2. - t).max(0.));
            let n = 240;
            (0..n)
                .map(|k| {
                    let th = std::f64::consts::TAU * k as f64 / n as f64;
                    (ro * th, [ro * th.cos(), ro * th.sin()], [ri * th.cos(), ri * th.sin()])
                })
                .collect()
        }
        Secao::Retangular { b, h, t } => {
            let (hb, hh) = (b / 2., h / 2.);
            let ti = t.min(hb.min(hh) - 1e-9).max(0.);
            let (ib, ih) = (hb - ti, hh - ti);
            let vo = [[hb, 0.], [hb, hh], [-hb, hh], [-hb, -hh], [hb, -hh], [hb, 0.]];
            let vi = [[ib, 0.], [ib, ih], [-ib, ih], [-ib, -ih], [ib, -ih], [ib, 0.]];
            let mut out = vec![];
            let mut s = 0.;
            for k in 0..5 {
                let (a, c) = (vo[k], vo[k + 1]);
                let comp = (c[0] - a[0]).hypot(c[1] - a[1]);
                let n = ((comp / 2.).ceil() as usize).clamp(8, 60);
                for i in 0..n {
                    let f = i as f64 / n as f64;
                    let lerp = |p: [f64; 2], q: [f64; 2]| [p[0] + (q[0] - p[0]) * f, p[1] + (q[1] - p[1]) * f];
                    out.push((s + comp * f, lerp(vo[k], vo[k + 1]), lerp(vi[k], vi[k + 1])));
                }
                s += comp;
            }
            out
        }
    }
}

/// Saída do raio `q + λ·u` do sólido convexo do hospedeiro (engordado por `g`): o maior `λ` dentro
/// dele e a normal externa nesse ponto. `None` se o raio não passa pelo sólido.
fn saida(h: &Membro, recuo: f64, g: f64, q: V3, u: V3) -> Option<(f64, V3)> {
    let r = geo3d::sub(q, h.ponto);
    let o = [geo3d::dot(r, h.x) - h.centro[0], geo3d::dot(r, h.y) - h.centro[1], geo3d::dot(r, h.para_dentro)];
    let d = [geo3d::dot(u, h.x), geo3d::dot(u, h.y), geo3d::dot(u, h.para_dentro)];
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    let mut n_hi = [0.; 3];
    // faixa de uma coordenada: a face de saída vira a normal se apertar `hi`
    let faixa = |k: usize, min: f64, max: f64, lo: &mut f64, hi: &mut f64, n_hi: &mut [f64; 3]| -> bool {
        if d[k].abs() < 1e-12 {
            return o[k] >= min && o[k] <= max;
        }
        let (t0, t1) = ((min - o[k]) / d[k], (max - o[k]) / d[k]);
        let (near, far, sinal) = if d[k] > 0. { (t0, t1, 1.) } else { (t1, t0, -1.) };
        *lo = lo.max(near);
        if far < *hi {
            *hi = far;
            *n_hi = [0.; 3];
            n_hi[k] = sinal;
        }
        true
    };
    match h.secao {
        Secao::Retangular { b, h: alt, .. } => {
            if !faixa(0, -b / 2. - g, b / 2. + g, &mut lo, &mut hi, &mut n_hi) || !faixa(1, -alt / 2. - g, alt / 2. + g, &mut lo, &mut hi, &mut n_hi) {
                return None;
            }
        }
        Secao::Redonda { d: dia, .. } => {
            let raio = dia / 2. + g;
            let a = d[0] * d[0] + d[1] * d[1];
            let bq = 2. * (o[0] * d[0] + o[1] * d[1]);
            let c = o[0] * o[0] + o[1] * o[1] - raio * raio;
            if a < 1e-12 {
                if c > 0. {
                    return None;
                }
            } else {
                let disc = bq * bq - 4. * a * c;
                if disc < 0. {
                    return None;
                }
                let sq = disc.sqrt();
                let (t0, t1) = ((-bq - sq) / (2. * a), (-bq + sq) / (2. * a));
                lo = lo.max(t0);
                if t1 < hi {
                    hi = t1;
                    let (px, py) = (o[0] + t1 * d[0], o[1] + t1 * d[1]);
                    let l = px.hypot(py).max(1e-12);
                    n_hi = [px / l, py / l, 0.];
                }
            }
        }
    }
    if !faixa(2, -recuo, h.comprimento + g, &mut lo, &mut hi, &mut n_hi) || lo > hi || !hi.is_finite() {
        return None;
    }
    let n = geo3d::add(geo3d::add(geo3d::mul(h.x, n_hi[0]), geo3d::mul(h.y, n_hi[1])), geo3d::mul(h.para_dentro, n_hi[2]));
    Some((hi, n))
}

/// Calcula a curva de corte da ponta de `m` contra os hospedeiros (o de maior `λ` vence em cada
/// pele). Erra se o eixo for paralelo a algum hospedeiro.
pub fn calcular(m: &Membro, hospedeiros: &[Membro], opc: Opcoes) -> Result<CorteEncaixe, String> {
    let u = geo3d::unit(m.para_dentro).ok_or("membro sem direção")?;
    for h in hospedeiros {
        let hz = geo3d::unit(h.para_dentro).ok_or("hospedeiro sem direção")?;
        if geo3d::dot(u, hz).abs() > 1. - 1e-6 {
            return Err("os eixos são paralelos: não há corte de encaixe".into());
        }
    }
    if hospedeiros.is_empty() {
        return Err("sem hospedeiro".into());
    }
    let t = m.secao.parede();
    let recuo = m.secao.meia() + 1.;
    let at = |pt: [f64; 2], hs: &[Membro]| -> Option<(f64, V3)> {
        let q = geo3d::add(m.ponto, geo3d::add(geo3d::mul(m.x, m.centro[0] + pt[0]), geo3d::mul(m.y, m.centro[1] + pt[1])));
        hs.iter().filter_map(|h| saida(h, recuo.max(h.secao.meia()), opc.folga, q, u)).max_by(|a, b| a.0.total_cmp(&b.0))
    };
    let mut pontos = vec![];
    for (s, po, pi) in amostras(m.secao) {
        let (e, i) = (at(po, hospedeiros), at(pi, hospedeiros));
        let q_ext = geo3d::add(m.ponto, geo3d::add(geo3d::mul(m.x, m.centro[0] + po[0]), geo3d::mul(m.y, m.centro[1] + po[1])));
        let (le, li, ne, ni, livre) = match (e, i) {
            (Some(a), Some(b)) => (a.0, b.0, a.1, b.1, false),
            (Some(a), None) => (a.0, a.0, a.1, a.1, false),
            (None, Some(b)) => (b.0, b.0, b.1, b.1, false),
            (None, None) => (0., 0., [0.; 3], [0.; 3], true),
        };
        // seno do ângulo entre o eixo e a superfície = |cos| entre o eixo e a normal
        let cos = |n: V3| geo3d::dot(u, n).abs();
        let bisel_ideal = if t > 1e-9 { (le - li).atan2(t).to_degrees() } else { 0. };
        let (lambda, parede, folga_raiz, invasao, bisel) = if livre {
            (0., Parede::Livre, 0., 0., 0.)
        } else if !opc.perpendicular {
            // a face acompanha o hospedeiro: a pele externa define a curva
            (le, Parede::Externa, 0., 0., bisel_ideal)
        } else {
            let (lam, par) = match opc.modo {
                ModoParede::Automatico => {
                    if le >= li {
                        (le, Parede::Externa)
                    } else {
                        (li, Parede::Interna)
                    }
                }
                ModoParede::Externa => (le, Parede::Externa),
                ModoParede::Interna => (li, Parede::Interna),
                ModoParede::Media => ((le + li) / 2., Parede::Media),
            };
            let (hi_l, hi_n, lo_l, lo_n) = if le >= li { (le, ne, li, ni) } else { (li, ni, le, ne) };
            let invasao = (hi_l - lam).max(0.) * cos(hi_n);
            let folga = (lam - lo_l).max(0.) * cos(lo_n);
            (lam, par, folga, invasao, 0.)
        };
        pontos.push(PontoCorte {
            s,
            lambda,
            parede,
            folga_raiz,
            invasao,
            bisel,
            bisel_ideal,
            excede: opc.perpendicular && folga_raiz > opc.orcamento + 1e-9,
            p3: geo3d::add(q_ext, geo3d::mul(u, lambda)),
            lambda_ext: le,
            lambda_int: li,
        });
    }
    Ok(CorteEncaixe { secao: m.secao, opc, pontos, perimetro: m.secao.perimetro() })
}

// ----------------------------------------------------------------------------- tubo aberto

/// Contorno plano do membro com os encaixes das pontas, no mesmo formato do tubo aberto das
/// dobras: X ao longo do tubo a partir do nó da ponta 1, Y no perímetro externo, curva de corte na
/// camada `CORTE`, o tubo desenrolado em `CONTORNO` e os textos em `COTA`. A ponta 2 usa a mesma
/// seção, medida a partir do nó dela (`comprimento` é a distância entre os nós). As partes acima do
/// orçamento ganham um traço de `MARCACAO` paralelo à curva.
pub fn tubo_aberto(titulo: &str, comprimento: f64, p1: Option<&CorteEncaixe>, p2: Option<&CorteEncaixe>) -> Desenho {
    let mut d = Desenho { titulo: titulo.to_string(), ..Default::default() };
    let sec = p1.or(p2).map(|c| c.secao);
    let Some(sec) = sec else { return d };
    let per = sec.perimetro();
    let xmin = [p1.map(|c| c.lambda_min()), p2.map(|c| comprimento - c.lambda_max())].into_iter().flatten().fold(0f64, f64::min);
    d.retangulo(Camada::Contorno, xmin, 0., comprimento.max(p2.map(|c| comprimento - c.lambda_min()).unwrap_or(0.)), per);
    if let Secao::Retangular { b, h, .. } = sec {
        // arestas entre as faces, a partir do meio da face +x
        let mut s = h / 2.;
        for lado in [b, h, b] {
            d.linha(Camada::Contorno, [xmin, s], [comprimento, s], true);
            s += lado;
            if s >= per {
                break;
            }
        }
        d.linha(Camada::Contorno, [xmin, s], [comprimento, s], true);
    }
    let curva = |c: &CorteEncaixe, invertida: bool| -> Vec<[f64; 2]> {
        let x = |l: f64| if invertida { comprimento - l } else { l };
        let mut pts: Vec<[f64; 2]> = c.pontos.iter().map(|p| [x(p.lambda), p.s]).collect();
        // fecha na costura: o primeiro ponto de novo em y = perímetro
        if let Some(p0) = c.pontos.first() {
            pts.push([x(p0.lambda), c.perimetro]);
        }
        pts
    };
    for (c, inv, nome) in [(p1, false, "ponta 1"), (p2, true, "ponta 2")] {
        let Some(c) = c else { continue };
        d.poli(Camada::Corte, curva(c, inv), false);
        let x = |l: f64| if inv { comprimento - l } else { l };
        for (a, b) in c.trechos_acima() {
            let pts: Vec<[f64; 2]> = c.pontos.iter().filter(|p| p.s >= a - 1e-9 && p.s <= b + 1e-9).map(|p| [x(p.lambda), p.s]).collect();
            if pts.len() >= 2 {
                d.poli(Camada::Marcacao, pts, false);
            }
        }
        let [fe, fi, fm, _] = c.fracao_por_parede();
        let y = per + 1.5 + if inv { 0. } else { 4. };
        let x0 = if inv { comprimento - c.lambda_max() } else { c.lambda_min() };
        d.texto(
            Camada::Cota,
            [x0, y],
            2.5,
            format!(
                "{nome}: corte {:.2} mm · abertura máx. {:.2} mm (orçamento {:.2}) · parede ext. {:.0}% int. {:.0}% méd. {:.0}%{}",
                c.comprimento_corte(),
                c.folga_max(),
                c.opc.orcamento,
                fe * 100.,
                fi * 100.,
                fm * 100.,
                if c.opc.perpendicular { String::new() } else { format!(" · chanfro até {:.1}°", c.bisel_max()) }
            ),
        );
    }
    d.texto(
        Camada::Cota,
        [xmin, -5.],
        2.5,
        format!("Tubo desenrolado: {:.2} x {:.2} mm (pele externa; y = 0 no meio da face +x, sentido anti-horário; x medido do nó). Sem protótipo: a validar em peça real.", comprimento, per),
    );
    d
}
